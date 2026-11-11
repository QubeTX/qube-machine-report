//! Installer staging and the lifetime of its verified payload.
//!
//! Windows holds deny-delete directory handles and a deny-write/delete payload
//! handle from hashing until the installer has finished. Unix uses an atomic
//! mode-0700 directory under the root-owned system temporary directory. That
//! Unix boundary excludes other UIDs, not hostile processes with the same UID.

use std::cell::RefCell;
use std::fs::File;
use std::io::{self, Seek, SeekFrom};
use std::path::{Path, PathBuf};

pub(super) struct StagedInstaller {
    // Fields drop in declaration order: release the file before removing its
    // directory, whose Windows guards also need to close before cleanup.
    payload: RefCell<Option<File>>,
    dir: platform::Directory,
    path: PathBuf,
}

impl StagedInstaller {
    pub(super) fn new(extension: &str) -> io::Result<Self> {
        if extension.is_empty()
            || extension.len() > 16
            || !extension.bytes().all(|byte| byte.is_ascii_alphanumeric())
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "invalid staged installer extension",
            ));
        }
        let dir = platform::new_directory()?;
        let path = dir.path().join(format!("installer.{extension}"));
        Ok(Self {
            payload: RefCell::new(None),
            dir,
            path,
        })
    }

    pub(super) fn path(&self) -> &Path {
        &self.path
    }

    /// Hash the retained handle, never a handle that is discarded before launch.
    /// On Windows, acquiring this handle also fails if an existing writer would
    /// defeat its deny-write/delete share policy. Unix does not promise that a
    /// same-UID writer cannot modify the opened inode.
    pub(super) fn seal_and_hash(&self) -> io::Result<String> {
        use sha2::{Digest, Sha256};

        let mut payload = self.payload.borrow_mut();
        if payload.is_none() {
            *payload = Some(platform::open_payload(&self.path)?);
        }
        let file = payload.as_mut().expect("payload was opened above");
        file.seek(SeekFrom::Start(0))?;
        let mut hasher = Sha256::new();
        io::copy(file, &mut hasher)?;
        Ok(format!("{:x}", hasher.finalize()))
    }

    pub(super) fn close(self) -> io::Result<()> {
        let Self { payload, dir, .. } = self;
        drop(payload);
        dir.close()
    }
}

#[cfg(unix)]
mod platform {
    use super::*;
    use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};

    pub(super) type Directory = tempfile::TempDir;

    pub(super) fn new_directory() -> io::Result<Directory> {
        // Do not let TMPDIR redirect an updater's staging into an arbitrary
        // caller-selected tree. Canonicalization accepts macOS's /tmp alias.
        let base = Path::new("/tmp").canonicalize()?;
        for ancestor in base.ancestors() {
            let metadata = std::fs::symlink_metadata(ancestor)?;
            let writable = metadata.mode() & 0o022 != 0;
            let sticky_base = ancestor == base && metadata.mode() & 0o1000 != 0;
            if !metadata.is_dir() || metadata.uid() != 0 || (writable && !sticky_base) {
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "system temporary directory has an untrusted owner or permissions",
                ));
            }
        }
        tempfile::Builder::new()
            .prefix("tr300-update-")
            .permissions(std::fs::Permissions::from_mode(0o700))
            .tempdir_in(base)
    }

    pub(super) fn open_payload(path: &Path) -> io::Result<File> {
        let file = std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(path)?;
        if !file.metadata()?.is_file() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "staged installer is not a regular file",
            ));
        }
        Ok(file)
    }
}

#[cfg(windows)]
mod platform {
    use super::*;
    use std::mem::{size_of, zeroed};
    use std::os::windows::ffi::OsStrExt;
    use std::os::windows::fs::{MetadataExt, OpenOptionsExt};
    use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
    use std::path::{Component, Prefix};
    use winapi::shared::sddl::{
        ConvertSidToStringSidW, ConvertStringSecurityDescriptorToSecurityDescriptorW,
        SDDL_REVISION_1,
    };
    use winapi::um::fileapi::CreateDirectoryW;
    use winapi::um::knownfolders::FOLDERID_LocalAppData;
    use winapi::um::minwinbase::SECURITY_ATTRIBUTES;
    use winapi::um::processthreadsapi::{GetCurrentProcess, OpenProcessToken};
    use winapi::um::securitybaseapi::GetTokenInformation;
    use winapi::um::winbase::{
        LocalFree, FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT,
    };
    use winapi::um::winnt::{
        TokenElevation, TokenUser, FILE_ATTRIBUTE_REPARSE_POINT, FILE_SHARE_READ, FILE_SHARE_WRITE,
        TOKEN_ELEVATION, TOKEN_QUERY, TOKEN_USER,
    };

    pub(super) struct Directory {
        path: PathBuf,
        guards: Vec<File>,
    }

    impl Directory {
        pub(super) fn path(&self) -> &Path {
            &self.path
        }

        pub(super) fn close(mut self) -> io::Result<()> {
            self.guards.clear();
            let path = std::mem::take(&mut self.path);
            std::fs::remove_dir_all(path)
        }
    }

    impl Drop for Directory {
        fn drop(&mut self) {
            self.guards.clear();
            if !self.path.as_os_str().is_empty() {
                let _ = std::fs::remove_dir_all(&self.path);
            }
        }
    }

    struct LocalAllocation(*mut winapi::ctypes::c_void);

    impl Drop for LocalAllocation {
        fn drop(&mut self) {
            // SAFETY: these allocations come only from the two conversion APIs
            // below, which require LocalFree, and are freed exactly once.
            unsafe { LocalFree(self.0.cast()) };
        }
    }

    pub(super) fn new_directory() -> io::Result<Directory> {
        let token = process_token()?;
        let elevated = token_is_elevated(&token)?;
        let base = if elevated {
            // Global updates already run inside the narrow elevated worker.
            // A protected native Program Files base excludes medium-token
            // parent substitution rather than trusting inherited TMP/TEMP.
            crate::trusted_process::windows_program_files_x64()?
        } else {
            crate::trusted_process::windows_known_folder(&FOLDERID_LocalAppData)?
        };
        let mut guards = pin_directory_chain(&base)?;
        let (owner, user_ace) = if elevated {
            ("BA".to_string(), String::new())
        } else {
            let user = token_user_sid(&token)?;
            (user.clone(), format!("(A;OICI;FA;;;{user})"))
        };
        // Protected DACL at creation: there is no inherited-permission window.
        // Child payloads inherit only SYSTEM, Administrators, and (for a
        // non-elevated per-user update) the actual token's user SID. Set the
        // owner too: a token's configurable default owner is not necessarily
        // Administrators, and ownership carries implicit WRITE_DAC rights.
        let sddl = format!("O:{owner}D:P(A;OICI;FA;;;SY)(A;OICI;FA;;;BA){user_ace}");
        let descriptor = security_descriptor(&sddl)?;
        let created = tempfile::Builder::new()
            .prefix("tr300-update-")
            .make_in(&base, |path| create_directory(path, &descriptor))?;
        let ((), mut temporary_path) = created.into_parts();
        // The Builder supplies secure random names/retries; Directory owns
        // recursive cleanup because TempPath's file cleanup is inappropriate.
        temporary_path.disable_cleanup(true);
        let path = temporary_path.to_path_buf();
        let mut directory = Directory {
            path,
            guards: Vec::new(),
        };
        guards.push(pin_directory(directory.path())?);
        directory.guards = guards;
        Ok(directory)
    }

    fn process_token() -> io::Result<OwnedHandle> {
        let mut token = std::ptr::null_mut();
        // SAFETY: a valid pseudo process handle, output pointer and query-only
        // access are used; ownership transfers to OwnedHandle on success.
        if unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) } == 0 {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: OpenProcessToken returned an owned, nonnull token handle.
        Ok(unsafe { OwnedHandle::from_raw_handle(token.cast()) })
    }

    fn token_is_elevated(token: &OwnedHandle) -> io::Result<bool> {
        // SAFETY: TOKEN_ELEVATION is a POD Windows output structure.
        let mut elevation: TOKEN_ELEVATION = unsafe { zeroed() };
        let mut length = 0;
        // SAFETY: the buffer is correctly aligned and sized for this class.
        if unsafe {
            GetTokenInformation(
                token.as_raw_handle().cast(),
                TokenElevation,
                (&mut elevation as *mut TOKEN_ELEVATION).cast(),
                size_of::<TOKEN_ELEVATION>() as u32,
                &mut length,
            )
        } == 0
        {
            return Err(io::Error::last_os_error());
        }
        Ok(elevation.TokenIsElevated != 0)
    }

    fn token_user_sid(token: &OwnedHandle) -> io::Result<String> {
        let mut length = 0;
        // SAFETY: the first call asks only for the required byte count.
        unsafe {
            GetTokenInformation(
                token.as_raw_handle().cast(),
                TokenUser,
                std::ptr::null_mut(),
                0,
                &mut length,
            )
        };
        if length < size_of::<TOKEN_USER>() as u32 || length > 65_536 {
            return Err(io::Error::last_os_error());
        }
        // usize storage provides the TOKEN_USER pointer's required alignment.
        let mut storage = vec![0usize; (length as usize).div_ceil(size_of::<usize>())];
        // SAFETY: the buffer has the requested size and pointer alignment.
        if unsafe {
            GetTokenInformation(
                token.as_raw_handle().cast(),
                TokenUser,
                storage.as_mut_ptr().cast(),
                length,
                &mut length,
            )
        } == 0
        {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: the successful TokenUser query initialized this structure
        // and its SID remains inside storage for the following conversion.
        let user = unsafe { &*storage.as_ptr().cast::<TOKEN_USER>() };
        let mut text = std::ptr::null_mut();
        // SAFETY: user.User.Sid is valid while storage is alive; output is owned.
        if unsafe { ConvertSidToStringSidW(user.User.Sid, &mut text) } == 0 {
            return Err(io::Error::last_os_error());
        }
        let _allocation = LocalAllocation(text.cast());
        let mut count = 0;
        // SAFETY: ConvertSidToStringSidW returns a NUL-terminated SID string;
        // a SID's textual representation is bounded well below 256 UTF-16 units.
        while count < 256 && unsafe { *text.add(count) } != 0 {
            count += 1;
        }
        if count == 256 {
            return Err(io::Error::other("invalid token SID string"));
        }
        // SAFETY: count units were read inside the returned string allocation.
        String::from_utf16(unsafe { std::slice::from_raw_parts(text, count) })
            .map_err(|_| io::Error::other("invalid token SID encoding"))
    }

    fn security_descriptor(sddl: &str) -> io::Result<LocalAllocation> {
        let text: Vec<u16> = sddl.encode_utf16().chain(Some(0)).collect();
        let mut descriptor = std::ptr::null_mut();
        // SAFETY: text is NUL-terminated; output is released by LocalAllocation.
        if unsafe {
            ConvertStringSecurityDescriptorToSecurityDescriptorW(
                text.as_ptr(),
                u32::from(SDDL_REVISION_1),
                &mut descriptor,
                std::ptr::null_mut(),
            )
        } == 0
        {
            return Err(io::Error::last_os_error());
        }
        Ok(LocalAllocation(descriptor.cast()))
    }

    fn create_directory(path: &Path, descriptor: &LocalAllocation) -> io::Result<()> {
        let path: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
        let mut attributes = SECURITY_ATTRIBUTES {
            nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
            lpSecurityDescriptor: descriptor.0,
            bInheritHandle: 0,
        };
        // SAFETY: path is terminated and the descriptor stays valid for this
        // synchronous call. CreateDirectoryW atomically refuses an existing name.
        if unsafe { CreateDirectoryW(path.as_ptr(), &mut attributes) } == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }

    fn pin_directory_chain(path: &Path) -> io::Result<Vec<File>> {
        let mut components = path.components();
        let Some(Component::Prefix(prefix)) = components.next() else {
            return Err(io::Error::other(
                "staging base is not an absolute local path",
            ));
        };
        if !matches!(prefix.kind(), Prefix::Disk(_) | Prefix::VerbatimDisk(_))
            || components.next() != Some(Component::RootDir)
        {
            return Err(io::Error::other("staging base is not on a local drive"));
        }
        let mut current = PathBuf::from(prefix.as_os_str());
        current.push(std::path::MAIN_SEPARATOR.to_string());
        let mut guards = vec![pin_directory(&current)?];
        for component in components {
            let Component::Normal(name) = component else {
                return Err(io::Error::other("invalid staging base component"));
            };
            current.push(name);
            guards.push(pin_directory(&current)?);
        }
        Ok(guards)
    }

    fn pin_directory(path: &Path) -> io::Result<File> {
        let directory = std::fs::OpenOptions::new()
            .read(true)
            .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE)
            .custom_flags(FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT)
            .open(path)?;
        let metadata = directory.metadata()?;
        if !metadata.is_dir() || metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "staging ancestor is not a real directory",
            ));
        }
        Ok(directory)
    }

    pub(super) fn open_payload(path: &Path) -> io::Result<File> {
        let file = std::fs::OpenOptions::new()
            .read(true)
            .share_mode(FILE_SHARE_READ)
            .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT)
            .open(path)?;
        let metadata = file.metadata()?;
        if !metadata.is_file() || metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "staged installer is not a regular non-reparse file",
            ));
        }
        Ok(file)
    }
    #[cfg(test)]
    #[test]
    fn windows_stage_has_the_expected_owner_and_protected_dacl() {
        use winapi::um::securitybaseapi::{
            EqualSid, GetFileSecurityW, GetSecurityDescriptorControl, GetSecurityDescriptorDacl,
            GetSecurityDescriptorOwner,
        };
        use winapi::um::winnt::{
            DACL_SECURITY_INFORMATION, OWNER_SECURITY_INFORMATION, SE_DACL_PROTECTED,
        };

        let directory = new_directory().unwrap();
        let token = process_token().unwrap();
        let elevated = token_is_elevated(&token).unwrap();
        let expected_owner = if elevated {
            "S-1-5-32-544".to_string()
        } else {
            token_user_sid(&token).unwrap()
        };
        let expected_descriptor = security_descriptor(&format!("O:{expected_owner}")).unwrap();
        let requested = DACL_SECURITY_INFORMATION | OWNER_SECURITY_INFORMATION;
        let path: Vec<u16> = directory
            .path()
            .as_os_str()
            .encode_wide()
            .chain(Some(0))
            .collect();
        let mut length = 0;
        // SAFETY: this first call only obtains the required buffer length.
        unsafe {
            GetFileSecurityW(
                path.as_ptr(),
                requested,
                std::ptr::null_mut(),
                0,
                &mut length,
            )
        };
        assert!((1..65_536).contains(&length));
        let mut storage = vec![0usize; (length as usize).div_ceil(size_of::<usize>())];
        let descriptor = storage.as_mut_ptr().cast();
        // SAFETY: the allocation is aligned and at least length bytes long.
        assert_ne!(
            unsafe { GetFileSecurityW(path.as_ptr(), requested, descriptor, length, &mut length,) },
            0
        );
        let mut actual_owner = std::ptr::null_mut();
        let mut expected_owner = std::ptr::null_mut();
        let mut owner_defaulted = 0;
        // SAFETY: both initialized descriptors remain alive throughout these
        // queries and the returned SID comparison.
        assert_ne!(
            unsafe {
                GetSecurityDescriptorOwner(descriptor, &mut actual_owner, &mut owner_defaulted)
            },
            0
        );
        // SAFETY: expected_descriptor came from the validated SDDL conversion.
        assert_ne!(
            unsafe {
                GetSecurityDescriptorOwner(
                    expected_descriptor.0.cast(),
                    &mut expected_owner,
                    &mut owner_defaulted,
                )
            },
            0
        );
        assert!(!actual_owner.is_null() && !expected_owner.is_null());
        // SAFETY: both nonnull SIDs are owned by the live descriptors above.
        assert_ne!(unsafe { EqualSid(actual_owner, expected_owner) }, 0);
        let (mut control, mut revision) = (0, 0);
        // SAFETY: descriptor contains the successful query's security descriptor.
        assert_ne!(
            unsafe { GetSecurityDescriptorControl(descriptor, &mut control, &mut revision) },
            0
        );
        assert_ne!(control & SE_DACL_PROTECTED, 0);
        let (mut present, mut defaulted) = (0, 0);
        let mut dacl = std::ptr::null_mut();
        // SAFETY: output pointers are valid and descriptor remains alive.
        assert_ne!(
            unsafe {
                GetSecurityDescriptorDacl(descriptor, &mut present, &mut dacl, &mut defaulted)
            },
            0
        );
        assert_ne!(present, 0);
        assert!(!dacl.is_null(), "a NULL DACL would allow everyone access");
        // SAFETY: successful GetSecurityDescriptorDacl returned an ACL in storage.
        assert_eq!(unsafe { (*dacl).AceCount }, if elevated { 2 } else { 3 });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn staged_payload_hashes_verified_bytes_and_cleans_up() {
        let staged = StagedInstaller::new("exe").unwrap();
        let directory = staged.path().parent().unwrap().to_path_buf();
        std::fs::write(staged.path(), b"abc").unwrap();
        assert_eq!(
            staged.seal_and_hash().unwrap(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        staged.close().unwrap();
        assert!(!directory.exists());
    }

    #[cfg(unix)]
    #[test]
    fn unix_staging_is_private_and_ignores_caller_temp_environment() {
        use std::os::unix::fs::{MetadataExt, PermissionsExt};

        let staged = StagedInstaller::new("pkg").unwrap();
        let directory = staged.path().parent().unwrap();
        assert_eq!(
            directory.parent().unwrap(),
            Path::new("/tmp").canonicalize().unwrap()
        );
        let metadata = std::fs::metadata(directory).unwrap();
        assert_eq!(metadata.permissions().mode() & 0o777, 0o700);
        // SAFETY: geteuid has no preconditions or side effects.
        assert_eq!(metadata.uid(), unsafe { libc::geteuid() });
    }

    #[cfg(windows)]
    #[test]
    fn windows_verified_payload_and_ancestors_cannot_be_changed() {
        let staged = StagedInstaller::new("exe").unwrap();
        let path = staged.path();
        let directory = path.parent().unwrap();
        std::fs::write(path, b"verified installer fixture").unwrap();
        staged.seal_and_hash().unwrap();
        assert!(std::fs::OpenOptions::new().write(true).open(path).is_err());
        assert!(std::fs::remove_file(path).is_err());
        assert!(std::fs::rename(path, directory.join("renamed.exe")).is_err());
        assert!(std::fs::rename(directory, directory.with_extension("moved")).is_err());
        let base = directory.parent().unwrap();
        // The test never attempts to rename a real OS directory: validate its
        // held deny-delete handle by requesting conflicting DELETE access.
        use std::os::windows::fs::OpenOptionsExt;
        assert!(std::fs::OpenOptions::new()
            .access_mode(winapi::um::winnt::DELETE)
            .custom_flags(winapi::um::winbase::FILE_FLAG_BACKUP_SEMANTICS)
            .open(base)
            .is_err());
    }

    #[cfg(windows)]
    #[test]
    fn windows_sealing_refuses_an_existing_writable_handle() {
        let staged = StagedInstaller::new("msi").unwrap();
        let writer = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(staged.path())
            .unwrap();
        assert!(staged.seal_and_hash().is_err());
        drop(writer);
        assert!(staged.seal_and_hash().is_ok());
    }

    #[cfg(windows)]
    #[test]
    fn windows_native_executable_can_run_while_payload_is_sealed() {
        let staged = StagedInstaller::new("exe").unwrap();
        let source = crate::trusted_process::windows_system_directory()
            .unwrap()
            .join("where.exe");
        std::fs::copy(source, staged.path()).unwrap();
        staged.seal_and_hash().unwrap();
        let mut command = std::process::Command::new(staged.path());
        crate::trusted_process::configure_environment(&mut command).unwrap();
        let output = command
            .arg("/?")
            .current_dir(staged.path().parent().unwrap())
            .output()
            .unwrap();
        assert!(output.status.success(), "{output:?}");
        staged.close().unwrap();
    }
}
