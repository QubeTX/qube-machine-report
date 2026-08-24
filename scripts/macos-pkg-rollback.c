#ifdef __APPLE__
#define _DARWIN_C_SOURCE 1
#else
#define _DEFAULT_SOURCE 1
#endif
#define _POSIX_C_SOURCE 200809L

#include <errno.h>
#include <fcntl.h>
#include <grp.h>
#ifdef __APPLE__
#include <copyfile.h>
#endif
#include <limits.h>
#include <signal.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <sys/types.h>
#include <sys/wait.h>
#include <unistd.h>

#ifndef O_CLOEXEC
#define O_CLOEXEC 0
#endif

#ifndef O_DIRECTORY
#error "TR-300 PKG rollback requires O_DIRECTORY"
#endif

#ifndef O_NOFOLLOW
#error "TR-300 PKG rollback requires O_NOFOLLOW"
#endif

#ifndef AT_SYMLINK_NOFOLLOW
#error "TR-300 PKG rollback requires AT_SYMLINK_NOFOLLOW"
#endif

#define TR300_BINARY_MAX_BYTES (512LL * 1024LL * 1024LL)
#define TR300_RECEIPT_MAX_BYTES (4LL * 1024LL * 1024LL)
#define TR300_COPY_BUFFER_BYTES (64U * 1024U)
#define TR300_PREFLIGHT_MAGIC UINT64_C(0x545233303050464c)
#define TR300_PREFLIGHT_VERSION UINT64_C(1)

/*
 * Apple's package scripts run as root, while the managed Cargo paths belong to
 * the console user. Keep every directory and original file descriptor open
 * across validation, the unprivileged Rust dry-run, staging, and rollback.
 * Each Rust probe is first copied from a no-follow descriptor into a private,
 * non-writable executable directory, then launched only after supplementary
 * groups, gid, and uid have been dropped to the bound home owner.
 * Both managed files move to exclusive descriptor-relative siblings before
 * commit. Rollback writes only to exclusive siblings and commits with renameat
 * inside the already-bound user directories, so neither a changed parent path
 * nor a final-component symlink can redirect root writes.
 */

typedef struct {
    int fd;
    bool existed;
    struct stat identity;
    const char *relative_path;
} directory_binding;

typedef struct {
    directory_binding *directory;
    const char *name;
    const char *label;
    off_t maximum_size;
    bool executable;
    bool existed;
    int backup_fd;
    int identity_fd;
    bool mutated;
    bool staged;
    char staged_name[NAME_MAX + 1U];
    struct stat metadata;
} managed_snapshot;

typedef struct {
    int directory_fd;
    char directory_path[PATH_MAX];
    char executable_path[PATH_MAX];
} trusted_executable;

typedef struct {
    uint64_t magic;
    uint64_t version;
    uint64_t home_device;
    uint64_t home_inode;
    uint64_t home_uid;
    uint64_t home_gid;
} preflight_identity;

static volatile sig_atomic_t caught_signal = 0;
static volatile sig_atomic_t cleanup_pid = -1;

static void report_errno(const char *operation, const char *subject) {
    fprintf(stderr, "TR-300: %s %s failed: %s\n", operation, subject,
            strerror(errno));
}

static void close_if_open(int *fd) {
    if (*fd >= 0) {
        (void)close(*fd);
        *fd = -1;
    }
}

static bool same_identity(const struct stat *left, const struct stat *right) {
    return left->st_dev == right->st_dev && left->st_ino == right->st_ino &&
           left->st_uid == right->st_uid &&
           (left->st_mode & S_IFMT) == (right->st_mode & S_IFMT);
}

static bool same_snapshot_metadata(const struct stat *left,
                                   const struct stat *right) {
    if (!same_identity(left, right) || left->st_gid != right->st_gid ||
        left->st_nlink != right->st_nlink ||
        left->st_size != right->st_size || left->st_mode != right->st_mode) {
        return false;
    }
#ifdef __APPLE__
    return left->st_flags == right->st_flags &&
           left->st_mtimespec.tv_sec == right->st_mtimespec.tv_sec &&
           left->st_mtimespec.tv_nsec == right->st_mtimespec.tv_nsec &&
           left->st_ctimespec.tv_sec == right->st_ctimespec.tv_sec &&
           left->st_ctimespec.tv_nsec == right->st_ctimespec.tv_nsec;
#else
    return left->st_mtim.tv_sec == right->st_mtim.tv_sec &&
           left->st_mtim.tv_nsec == right->st_mtim.tv_nsec &&
           left->st_ctim.tv_sec == right->st_ctim.tv_sec &&
           left->st_ctim.tv_nsec == right->st_ctim.tv_nsec;
#endif
}

static bool same_post_rename_metadata(const struct stat *before,
                                      const struct stat *after) {
    if (!same_identity(before, after) || before->st_gid != after->st_gid ||
        before->st_nlink != after->st_nlink ||
        before->st_size != after->st_size || before->st_mode != after->st_mode) {
        return false;
    }
#ifdef __APPLE__
    return before->st_flags == after->st_flags &&
           before->st_mtimespec.tv_sec == after->st_mtimespec.tv_sec &&
           before->st_mtimespec.tv_nsec == after->st_mtimespec.tv_nsec;
#else
    return before->st_mtim.tv_sec == after->st_mtim.tv_sec &&
           before->st_mtim.tv_nsec == after->st_mtim.tv_nsec;
#endif
}

static int duplicate_cloexec(int fd) {
    int duplicate = dup(fd);
    if (duplicate < 0) {
        return -1;
    }
    if (fcntl(duplicate, F_SETFD, FD_CLOEXEC) < 0) {
        int saved_errno = errno;
        (void)close(duplicate);
        errno = saved_errno;
        return -1;
    }
    return duplicate;
}

static bool valid_component(const char *component) {
    return component[0] != '\0' && strcmp(component, ".") != 0 &&
           strcmp(component, "..") != 0 && strchr(component, '/') == NULL;
}

static int open_absolute_directory_nofollow(const char *path,
                                             struct stat *identity) {
    if (path == NULL || path[0] != '/' || strlen(path) >= PATH_MAX) {
        errno = EINVAL;
        return -1;
    }

    char path_copy[PATH_MAX];
    (void)snprintf(path_copy, sizeof(path_copy), "%s", path);

    int current = open("/", O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC);
    if (current < 0) {
        return -1;
    }

    char *cursor = path_copy + 1;
    char *save = NULL;
    for (char *component = strtok_r(cursor, "/", &save); component != NULL;
         component = strtok_r(NULL, "/", &save)) {
        if (!valid_component(component)) {
            (void)close(current);
            errno = EINVAL;
            return -1;
        }
        int next = openat(current, component,
                          O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC);
        if (next < 0) {
            int saved_errno = errno;
            (void)close(current);
            errno = saved_errno;
            return -1;
        }
        (void)close(current);
        current = next;
    }

    if (fstat(current, identity) < 0 || !S_ISDIR(identity->st_mode)) {
        int saved_errno = errno == 0 ? ENOTDIR : errno;
        (void)close(current);
        errno = saved_errno;
        return -1;
    }
    return current;
}

static int open_absolute_file_nofollow(const char *path,
                                       struct stat *identity) {
    if (path == NULL || path[0] != '/' || strlen(path) >= PATH_MAX) {
        errno = EINVAL;
        return -1;
    }

    char path_copy[PATH_MAX];
    (void)snprintf(path_copy, sizeof(path_copy), "%s", path);
    char *separator = strrchr(path_copy, '/');
    if (separator == NULL || separator[1] == '\0' ||
        !valid_component(separator + 1)) {
        errno = EINVAL;
        return -1;
    }
    char name[NAME_MAX + 1U];
    int name_count = snprintf(name, sizeof(name), "%s", separator + 1);
    if (name_count < 0 || (size_t)name_count >= sizeof(name)) {
        errno = ENAMETOOLONG;
        return -1;
    }
    if (separator == path_copy) {
        separator[1] = '\0';
    } else {
        *separator = '\0';
    }

    struct stat directory_identity;
    int directory_fd =
        open_absolute_directory_nofollow(path_copy, &directory_identity);
    if (directory_fd < 0) {
        return -1;
    }

    struct stat pathname_identity;
    int fd = -1;
    if (fstatat(directory_fd, name, &pathname_identity,
                AT_SYMLINK_NOFOLLOW) < 0 ||
        (fd = openat(directory_fd, name,
                     O_RDONLY | O_NOFOLLOW | O_CLOEXEC)) < 0 ||
        fstat(fd, identity) < 0 || !same_identity(&pathname_identity, identity)) {
        int saved_errno = errno == 0 ? ESTALE : errno;
        close_if_open(&fd);
        (void)close(directory_fd);
        errno = saved_errno;
        return -1;
    }
    (void)close(directory_fd);
    return fd;
}

static uid_t preflight_state_owner(void) {
#ifdef TR300_ROLLBACK_TESTING
    return geteuid();
#else
    return 0;
#endif
}

static int open_preflight_state_directory(const char *state_path, char *name,
                                          size_t name_size,
                                          struct stat *directory_identity) {
    if (state_path == NULL || state_path[0] != '/' ||
        strlen(state_path) >= PATH_MAX) {
        errno = EINVAL;
        return -1;
    }

    char path_copy[PATH_MAX];
    (void)snprintf(path_copy, sizeof(path_copy), "%s", state_path);
    char *separator = strrchr(path_copy, '/');
    if (separator == NULL || separator[1] == '\0' ||
        !valid_component(separator + 1)) {
        errno = EINVAL;
        return -1;
    }
    int name_count = snprintf(name, name_size, "%s", separator + 1);
    if (name_count < 0 || (size_t)name_count >= name_size) {
        errno = ENAMETOOLONG;
        return -1;
    }
    if (separator == path_copy) {
        separator[1] = '\0';
    } else {
        *separator = '\0';
    }

    int directory_fd =
        open_absolute_directory_nofollow(path_copy, directory_identity);
    if (directory_fd < 0) {
        return -1;
    }
    if (directory_identity->st_uid != preflight_state_owner() ||
        (directory_identity->st_mode & 0022) != 0) {
        (void)close(directory_fd);
        errno = EPERM;
        return -1;
    }
    return directory_fd;
}

static bool acceptable_preflight_state_metadata(const struct stat *metadata) {
    return S_ISREG(metadata->st_mode) &&
           metadata->st_uid == preflight_state_owner() &&
           metadata->st_nlink == 1 &&
           metadata->st_size == (off_t)sizeof(preflight_identity) &&
           (metadata->st_mode & 07777) == 0600
#ifdef __APPLE__
           && (metadata->st_flags &
               (UF_IMMUTABLE | UF_APPEND | SF_IMMUTABLE | SF_APPEND)) == 0
#endif
        ;
}

static int write_preflight_state(const char *state_path,
                                 const struct stat *home_identity) {
    char state_name[NAME_MAX + 1U];
    struct stat directory_identity;
    int directory_fd = open_preflight_state_directory(
        state_path, state_name, sizeof(state_name), &directory_identity);
    if (directory_fd < 0) {
        return -1;
    }

    int state_fd = openat(directory_fd, state_name,
                          O_RDWR | O_CREAT | O_EXCL | O_NOFOLLOW | O_CLOEXEC,
                          0600);
    if (state_fd < 0) {
        int saved_errno = errno;
        (void)close(directory_fd);
        errno = saved_errno;
        return -1;
    }

    const preflight_identity state = {
        .magic = TR300_PREFLIGHT_MAGIC,
        .version = TR300_PREFLIGHT_VERSION,
        .home_device = (uint64_t)home_identity->st_dev,
        .home_inode = (uint64_t)home_identity->st_ino,
        .home_uid = (uint64_t)home_identity->st_uid,
        .home_gid = (uint64_t)home_identity->st_gid,
    };
    const unsigned char *cursor = (const unsigned char *)&state;
    size_t remaining = sizeof(state);
    while (remaining > 0) {
        ssize_t count = write(state_fd, cursor, remaining);
        if (count < 0 && errno == EINTR) {
            continue;
        }
        if (count <= 0) {
            errno = count == 0 ? EIO : errno;
            goto failure;
        }
        cursor += count;
        remaining -= (size_t)count;
    }

    struct stat descriptor_identity;
    struct stat pathname_identity;
    if (fsync(state_fd) < 0 || fstat(state_fd, &descriptor_identity) < 0 ||
        !acceptable_preflight_state_metadata(&descriptor_identity) ||
        fstatat(directory_fd, state_name, &pathname_identity,
                AT_SYMLINK_NOFOLLOW) < 0 ||
        !same_identity(&descriptor_identity, &pathname_identity) ||
        fsync(directory_fd) < 0) {
        if (errno == 0) {
            errno = ESTALE;
        }
        goto failure;
    }
    if (close(state_fd) < 0) {
        int saved_errno = errno;
        (void)unlinkat(directory_fd, state_name, 0);
        (void)close(directory_fd);
        errno = saved_errno;
        return -1;
    }
    return close(directory_fd);

failure: {
        int saved_errno = errno;
        (void)close(state_fd);
        (void)unlinkat(directory_fd, state_name, 0);
        (void)fsync(directory_fd);
        (void)close(directory_fd);
        errno = saved_errno;
        return -1;
    }
}

static int consume_preflight_state(const char *state_path,
                                   const struct stat *home_identity) {
    char state_name[NAME_MAX + 1U];
    struct stat directory_identity;
    int directory_fd = open_preflight_state_directory(
        state_path, state_name, sizeof(state_name), &directory_identity);
    if (directory_fd < 0) {
        return -1;
    }

    struct stat pathname_before;
    int state_fd = -1;
    if (fstatat(directory_fd, state_name, &pathname_before,
                AT_SYMLINK_NOFOLLOW) < 0 ||
        !acceptable_preflight_state_metadata(&pathname_before) ||
        (state_fd = openat(directory_fd, state_name,
                           O_RDONLY | O_NOFOLLOW | O_CLOEXEC)) < 0) {
        int saved_errno = errno == 0 ? EPERM : errno;
        close_if_open(&state_fd);
        (void)close(directory_fd);
        errno = saved_errno;
        return -1;
    }

    struct stat descriptor_before;
    struct stat descriptor_after;
    struct stat pathname_after;
    preflight_identity state;
    unsigned char extra;
    if (fstat(state_fd, &descriptor_before) < 0 ||
        !acceptable_preflight_state_metadata(&descriptor_before) ||
        !same_identity(&pathname_before, &descriptor_before)) {
        int saved_errno = errno == 0 ? ESTALE : errno;
        (void)close(state_fd);
        (void)close(directory_fd);
        errno = saved_errno;
        return -1;
    }
    ssize_t state_count;
    do {
        state_count = pread(state_fd, &state, sizeof(state), 0);
    } while (state_count < 0 && errno == EINTR);
    ssize_t extra_count;
    do {
        extra_count = pread(state_fd, &extra, sizeof(extra), sizeof(state));
    } while (extra_count < 0 && errno == EINTR);

    int result = 0;
    if (state_count != (ssize_t)sizeof(state) || extra_count != 0 ||
        fstat(state_fd, &descriptor_after) < 0 ||
        !same_snapshot_metadata(&descriptor_before, &descriptor_after) ||
        fstatat(directory_fd, state_name, &pathname_after,
                AT_SYMLINK_NOFOLLOW) < 0 ||
        !same_identity(&descriptor_after, &pathname_after) ||
        state.magic != TR300_PREFLIGHT_MAGIC ||
        state.version != TR300_PREFLIGHT_VERSION ||
        state.home_device != (uint64_t)home_identity->st_dev ||
        state.home_inode != (uint64_t)home_identity->st_ino ||
        state.home_uid != (uint64_t)home_identity->st_uid ||
        state.home_gid != (uint64_t)home_identity->st_gid) {
        errno = ESTALE;
        result = -1;
    }
    if (result == 0 &&
        (unlinkat(directory_fd, state_name, 0) < 0 ||
         fsync(directory_fd) < 0 ||
         fstat(state_fd, &descriptor_after) < 0 ||
         descriptor_after.st_nlink != 0)) {
        result = -1;
    }
    int saved_errno = errno;
    (void)close(state_fd);
    (void)close(directory_fd);
    errno = saved_errno;
    return result;
}

static int open_relative_directory_nofollow(int start_fd, const char *path,
                                            uid_t expected_uid,
                                            struct stat *identity) {
    if (path == NULL || path[0] == '/' || strlen(path) >= PATH_MAX) {
        errno = EINVAL;
        return -1;
    }

    char path_copy[PATH_MAX];
    (void)snprintf(path_copy, sizeof(path_copy), "%s", path);
    int current = duplicate_cloexec(start_fd);
    if (current < 0) {
        return -1;
    }

    char *save = NULL;
    for (char *component = strtok_r(path_copy, "/", &save); component != NULL;
         component = strtok_r(NULL, "/", &save)) {
        if (!valid_component(component)) {
            (void)close(current);
            errno = EINVAL;
            return -1;
        }
        int next = openat(current, component,
                          O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC);
        if (next < 0) {
            int saved_errno = errno;
            (void)close(current);
            errno = saved_errno;
            return -1;
        }
        struct stat component_identity;
        if (fstat(next, &component_identity) < 0 ||
            !S_ISDIR(component_identity.st_mode) ||
            component_identity.st_uid != expected_uid) {
            int saved_errno = errno == 0 ? EPERM : errno;
            (void)close(next);
            (void)close(current);
            errno = saved_errno;
            return -1;
        }
        (void)close(current);
        current = next;
    }

    if (fstat(current, identity) < 0) {
        int saved_errno = errno;
        (void)close(current);
        errno = saved_errno;
        return -1;
    }
    return current;
}

static int bind_optional_directory(int home_fd, uid_t expected_uid,
                                   directory_binding *binding) {
    binding->fd = open_relative_directory_nofollow(
        home_fd, binding->relative_path, expected_uid, &binding->identity);
    if (binding->fd >= 0) {
        binding->existed = true;
        return 0;
    }
    if (errno == ENOENT) {
        binding->existed = false;
        return 0;
    }
    report_errno("binding managed directory", binding->relative_path);
    return -1;
}

static bool acceptable_file_metadata(const struct stat *metadata,
                                     uid_t expected_uid, off_t maximum_size,
                                     bool executable) {
    if (!S_ISREG(metadata->st_mode) || metadata->st_uid != expected_uid ||
        metadata->st_nlink != 1 || metadata->st_size <= 0 ||
        metadata->st_size > maximum_size ||
        (metadata->st_mode & 07000) != 0
#ifdef __APPLE__
        || (metadata->st_flags &
            (UF_IMMUTABLE | UF_APPEND | SF_IMMUTABLE | SF_APPEND)) != 0
#endif
    ) {
        return false;
    }
    return !executable || (metadata->st_mode & S_IXUSR) != 0;
}

#ifdef __APPLE__
static bool is_macho_binary(int fd) {
    unsigned char magic[4];
    ssize_t count = pread(fd, magic, sizeof(magic), 0);
    if (count != (ssize_t)sizeof(magic)) {
        return false;
    }
    const uint32_t value = ((uint32_t)magic[0] << 24U) |
                           ((uint32_t)magic[1] << 16U) |
                           ((uint32_t)magic[2] << 8U) | (uint32_t)magic[3];
    return value == 0xfeedfaceU || value == 0xcefaedfeU ||
           value == 0xfeedfacfU || value == 0xcffaedfeU ||
           value == 0xcafebabeU || value == 0xbebafecaU ||
           value == 0xcafebabfU || value == 0xbfbafecaU;
}
#endif

static int inspect_managed_file(managed_snapshot *snapshot, uid_t expected_uid,
                                int *source_fd) {
    *source_fd = -1;
    snapshot->existed = false;
    snapshot->backup_fd = -1;
    snapshot->identity_fd = -1;
    snapshot->mutated = false;
    snapshot->staged = false;
    snapshot->staged_name[0] = '\0';

    if (!snapshot->directory->existed) {
        return 0;
    }

    struct stat pathname_identity;
    if (fstatat(snapshot->directory->fd, snapshot->name, &pathname_identity,
                AT_SYMLINK_NOFOLLOW) < 0) {
        if (errno == ENOENT) {
            return 0;
        }
        report_errno("inspecting", snapshot->label);
        return -1;
    }
    if (!acceptable_file_metadata(&pathname_identity, expected_uid,
                                  snapshot->maximum_size,
                                  snapshot->executable)) {
        fprintf(stderr,
                "TR-300: %s is not a single regular, expected-owner%s file; "
                "preserving it and rejecting PKG takeover.\n",
                snapshot->label,
                snapshot->executable ? ", executable" : "");
        errno = EPERM;
        return -1;
    }

    int fd = openat(snapshot->directory->fd, snapshot->name,
                    O_RDONLY | O_NOFOLLOW | O_CLOEXEC);
    if (fd < 0) {
        report_errno("opening", snapshot->label);
        return -1;
    }
    struct stat descriptor_identity;
    struct stat confirmed_pathname_identity;
    if (fstat(fd, &descriptor_identity) < 0 ||
        fstatat(snapshot->directory->fd, snapshot->name,
                &confirmed_pathname_identity, AT_SYMLINK_NOFOLLOW) < 0 ||
        !same_identity(&pathname_identity, &descriptor_identity) ||
        !same_identity(&descriptor_identity, &confirmed_pathname_identity)) {
        int saved_errno = errno == 0 ? ESTALE : errno;
        (void)close(fd);
        errno = saved_errno;
        fprintf(stderr,
                "TR-300: %s changed while its identity was being bound; "
                "preserving it and rejecting PKG takeover.\n",
                snapshot->label);
        return -1;
    }
#ifdef __APPLE__
    if (snapshot->executable && !is_macho_binary(fd)) {
        (void)close(fd);
        errno = ENOEXEC;
        fprintf(stderr,
                "TR-300: the prior managed binary is not a Mach-O executable; "
                "preserving it and rejecting PKG takeover.\n");
        return -1;
    }
#endif

    snapshot->metadata = descriptor_identity;
    snapshot->existed = true;
    *source_fd = fd;
    return 0;
}

static int create_anonymous_backup(void) {
#ifdef __APPLE__
    char path[] = "/private/tmp/tr300-pkg-rollback.XXXXXXXX";
#else
    char path[] = "/tmp/tr300-pkg-rollback.XXXXXXXX";
#endif
    int fd = mkstemp(path);
    if (fd < 0) {
        return -1;
    }
    if (fcntl(fd, F_SETFD, FD_CLOEXEC) < 0 || fchmod(fd, 0600) < 0 ||
        unlink(path) < 0) {
        int saved_errno = errno;
        (void)close(fd);
        (void)unlink(path);
        errno = saved_errno;
        return -1;
    }
    return fd;
}

static int copy_descriptor(int source_fd, int destination_fd,
                           off_t expected_size, off_t maximum_size) {
    if (expected_size <= 0 || expected_size > maximum_size) {
        errno = EFBIG;
        return -1;
    }
    if (lseek(source_fd, 0, SEEK_SET) < 0 ||
        ftruncate(destination_fd, 0) < 0 ||
        lseek(destination_fd, 0, SEEK_SET) < 0) {
        return -1;
    }
    unsigned char *buffer = malloc(TR300_COPY_BUFFER_BYTES);
    if (buffer == NULL) {
        errno = ENOMEM;
        return -1;
    }
    off_t copied = 0;
    while (copied < expected_size) {
        off_t remaining = expected_size - copied;
        size_t request = remaining < (off_t)TR300_COPY_BUFFER_BYTES
                             ? (size_t)remaining
                             : TR300_COPY_BUFFER_BYTES;
        ssize_t read_count = read(source_fd, buffer, request);
        if (read_count < 0) {
            if (errno == EINTR) {
                continue;
            }
            free(buffer);
            return -1;
        }
        if (read_count == 0) {
            free(buffer);
            errno = ESTALE;
            return -1;
        }
        ssize_t offset = 0;
        while (offset < read_count) {
            ssize_t written = write(destination_fd, buffer + offset,
                                    (size_t)(read_count - offset));
            if (written < 0) {
                if (errno == EINTR) {
                    continue;
                }
                free(buffer);
                return -1;
            }
            if (written == 0) {
                free(buffer);
                errno = EIO;
                return -1;
            }
            offset += written;
        }
        copied += read_count;
    }
    unsigned char extra;
    ssize_t extra_count;
    do {
        extra_count = read(source_fd, &extra, sizeof(extra));
    } while (extra_count < 0 && errno == EINTR);
    if (extra_count != 0) {
        free(buffer);
        errno = extra_count > 0 ? EFBIG : errno;
        return -1;
    }
    free(buffer);
    return fsync(destination_fd);
}

static int copy_descriptor_metadata(int source_fd, int destination_fd) {
#ifdef __APPLE__
    return fcopyfile(source_fd, destination_fd, NULL, COPYFILE_METADATA);
#else
    (void)source_fd;
    (void)destination_fd;
    return 0;
#endif
}

static int snapshot_managed_file(managed_snapshot *snapshot,
                                 uid_t expected_uid) {
    int source_fd = -1;
    if (inspect_managed_file(snapshot, expected_uid, &source_fd) < 0) {
        return -1;
    }
    if (!snapshot->existed) {
        return 0;
    }

    snapshot->backup_fd = create_anonymous_backup();
    if (snapshot->backup_fd < 0 ||
        copy_descriptor(source_fd, snapshot->backup_fd,
                        snapshot->metadata.st_size,
                        snapshot->maximum_size) < 0 ||
        copy_descriptor_metadata(source_fd, snapshot->backup_fd) < 0 ||
        fsync(snapshot->backup_fd) < 0) {
        int saved_errno = errno;
        report_errno("snapshotting", snapshot->label);
        close_if_open(&source_fd);
        close_if_open(&snapshot->backup_fd);
        errno = saved_errno;
        return -1;
    }
    struct stat after_copy;
    if (fstat(source_fd, &after_copy) < 0 ||
        !same_snapshot_metadata(&snapshot->metadata, &after_copy)) {
        int saved_errno = errno == 0 ? ESTALE : errno;
        fprintf(stderr,
                "TR-300: %s changed while it was being snapshotted; "
                "preserving it and rejecting PKG takeover.\n",
                snapshot->label);
        close_if_open(&source_fd);
        close_if_open(&snapshot->backup_fd);
        errno = saved_errno;
        return -1;
    }
    snapshot->identity_fd = source_fd;
    return 0;
}

static int random_u64(uint64_t *value) {
    int fd = open("/dev/urandom", O_RDONLY | O_CLOEXEC);
    if (fd < 0) {
        return -1;
    }
    unsigned char *cursor = (unsigned char *)value;
    size_t remaining = sizeof(*value);
    while (remaining > 0) {
        ssize_t count = read(fd, cursor, remaining);
        if (count < 0 && errno == EINTR) {
            continue;
        }
        if (count <= 0) {
            int saved_errno = count == 0 ? EIO : errno;
            (void)close(fd);
            errno = saved_errno;
            return -1;
        }
        cursor += count;
        remaining -= (size_t)count;
    }
    return close(fd);
}

static int create_private_sibling(int directory_fd, char *name,
                                  size_t name_size) {
    for (unsigned int attempt = 0; attempt < 128U; ++attempt) {
        uint64_t random_value;
        if (random_u64(&random_value) < 0) {
            return -1;
        }
        int count = snprintf(name, name_size,
                             ".tr300-pkg-rollback-%ld-%016llx", (long)getpid(),
                             (unsigned long long)random_value);
        if (count < 0 || (size_t)count >= name_size) {
            errno = ENAMETOOLONG;
            return -1;
        }
        int fd = openat(directory_fd, name,
                        O_WRONLY | O_CREAT | O_EXCL | O_NOFOLLOW | O_CLOEXEC,
                        0600);
        if (fd >= 0) {
            return fd;
        }
        if (errno != EEXIST) {
            return -1;
        }
    }
    errno = EEXIST;
    return -1;
}

static int descriptors_equal(int left_fd, int right_fd, off_t expected_size) {
    unsigned char *left = malloc(TR300_COPY_BUFFER_BYTES);
    unsigned char *right = malloc(TR300_COPY_BUFFER_BYTES);
    if (left == NULL || right == NULL) {
        free(left);
        free(right);
        errno = ENOMEM;
        return -1;
    }

    off_t offset = 0;
    while (offset < expected_size) {
        off_t remaining = expected_size - offset;
        size_t request = remaining < (off_t)TR300_COPY_BUFFER_BYTES
                             ? (size_t)remaining
                             : TR300_COPY_BUFFER_BYTES;
        ssize_t left_count;
        do {
            left_count = pread(left_fd, left, request, offset);
        } while (left_count < 0 && errno == EINTR);
        ssize_t right_count;
        do {
            right_count = pread(right_fd, right, request, offset);
        } while (right_count < 0 && errno == EINTR);
        if (left_count <= 0 || right_count != left_count ||
            memcmp(left, right, (size_t)left_count) != 0) {
            free(left);
            free(right);
            errno = ESTALE;
            return -1;
        }
        offset += left_count;
    }
    free(left);
    free(right);
    return 0;
}

static bool acceptable_probe_metadata(const struct stat *metadata,
                                      uid_t expected_uid) {
#ifdef TR300_ROLLBACK_TESTING
    bool owner_matches =
        metadata->st_uid == 0 || metadata->st_uid == expected_uid;
#else
    (void)expected_uid;
    bool owner_matches = metadata->st_uid == 0;
#endif
    if (!S_ISREG(metadata->st_mode) || !owner_matches ||
        metadata->st_nlink != 1 || metadata->st_size <= 0 ||
        metadata->st_size > TR300_BINARY_MAX_BYTES ||
        (metadata->st_mode & S_IXUSR) == 0 ||
        (metadata->st_mode & 07000) != 0 ||
        (metadata->st_mode & 0022) != 0
#ifdef __APPLE__
        || (metadata->st_flags &
            (UF_IMMUTABLE | UF_APPEND | SF_IMMUTABLE | SF_APPEND)) != 0
#endif
    ) {
        return false;
    }
    return true;
}

static void initialize_trusted_executable(trusted_executable *executable) {
    executable->directory_fd = -1;
    executable->directory_path[0] = '\0';
    executable->executable_path[0] = '\0';
}

static void remove_trusted_executable(trusted_executable *executable) {
    if (executable->directory_fd >= 0) {
        (void)fchmod(executable->directory_fd, 0700);
        (void)unlinkat(executable->directory_fd, "probe", 0);
        (void)fsync(executable->directory_fd);
        close_if_open(&executable->directory_fd);
    }
    if (executable->directory_path[0] != '\0') {
        (void)rmdir(executable->directory_path);
    }
    executable->directory_path[0] = '\0';
    executable->executable_path[0] = '\0';
}

static int prepare_trusted_executable(const char *source_path,
                                      uid_t expected_uid,
                                      trusted_executable *executable) {
    initialize_trusted_executable(executable);

    struct stat source_metadata;
    int source_fd = open_absolute_file_nofollow(source_path, &source_metadata);
    if (source_fd < 0) {
        return -1;
    }
    if (!acceptable_probe_metadata(&source_metadata, expected_uid)
#if defined(__APPLE__) && !defined(TR300_ROLLBACK_TESTING)
        || !is_macho_binary(source_fd)
#endif
    ) {
        close_if_open(&source_fd);
        errno = EPERM;
        return -1;
    }

#ifdef __APPLE__
    char directory_template[] = "/private/tmp/tr300-pkg-probe.XXXXXXXX";
#else
    char directory_template[] = "/tmp/tr300-pkg-probe.XXXXXXXX";
#endif
    if (mkdtemp(directory_template) == NULL) {
        int saved_errno = errno;
        close_if_open(&source_fd);
        errno = saved_errno;
        return -1;
    }
    int directory_count = snprintf(executable->directory_path,
                                   sizeof(executable->directory_path), "%s",
                                   directory_template);
    if (directory_count < 0 ||
        (size_t)directory_count >= sizeof(executable->directory_path)) {
        int saved_errno = ENAMETOOLONG;
        (void)rmdir(directory_template);
        close_if_open(&source_fd);
        errno = saved_errno;
        return -1;
    }

    struct stat directory_metadata;
    executable->directory_fd = open(
        executable->directory_path,
        O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC);
    if (executable->directory_fd < 0 ||
        fstat(executable->directory_fd, &directory_metadata) < 0 ||
        !S_ISDIR(directory_metadata.st_mode) ||
        directory_metadata.st_uid != geteuid()) {
        int saved_errno = errno == 0 ? ESTALE : errno;
        close_if_open(&source_fd);
        remove_trusted_executable(executable);
        errno = saved_errno;
        return -1;
    }

    int destination_fd = openat(
        executable->directory_fd, "probe",
        O_RDWR | O_CREAT | O_EXCL | O_NOFOLLOW | O_CLOEXEC, 0500);
    struct stat source_after_copy;
    struct stat destination_metadata;
    if (destination_fd < 0 ||
        copy_descriptor(source_fd, destination_fd, source_metadata.st_size,
                        TR300_BINARY_MAX_BYTES) < 0 ||
        fchmod(destination_fd, 0555) < 0 || fsync(destination_fd) < 0 ||
        descriptors_equal(source_fd, destination_fd, source_metadata.st_size) <
            0 ||
        fstat(source_fd, &source_after_copy) < 0 ||
        !same_snapshot_metadata(&source_metadata, &source_after_copy) ||
        fstat(destination_fd, &destination_metadata) < 0 ||
        !S_ISREG(destination_metadata.st_mode) ||
        destination_metadata.st_uid != geteuid() ||
        destination_metadata.st_nlink != 1 ||
        destination_metadata.st_size != source_metadata.st_size ||
        (destination_metadata.st_mode & 0777) != 0555 ||
        fsync(executable->directory_fd) < 0 ||
        fchmod(executable->directory_fd, 0511) < 0) {
        int saved_errno = errno == 0 ? ESTALE : errno;
        if (destination_fd >= 0) {
            (void)close(destination_fd);
        }
        close_if_open(&source_fd);
        remove_trusted_executable(executable);
        errno = saved_errno;
        return -1;
    }
    int destination_close = close(destination_fd);
    destination_fd = -1;
    if (destination_close < 0) {
        int saved_errno = errno;
        close_if_open(&source_fd);
        remove_trusted_executable(executable);
        errno = saved_errno;
        return -1;
    }
    close_if_open(&source_fd);

    int executable_count = snprintf(
        executable->executable_path, sizeof(executable->executable_path),
        "%s/probe", executable->directory_path);
    if (executable_count < 0 ||
        (size_t)executable_count >= sizeof(executable->executable_path)) {
        remove_trusted_executable(executable);
        errno = ENAMETOOLONG;
        return -1;
    }
    return 0;
}

static int verify_snapshot_unchanged(const managed_snapshot *snapshot) {
    if (!snapshot->existed) {
        return 0;
    }
    struct stat descriptor_identity;
    struct stat pathname_identity;
    if (fstat(snapshot->identity_fd, &descriptor_identity) < 0 ||
        fstatat(snapshot->directory->fd, snapshot->name, &pathname_identity,
                AT_SYMLINK_NOFOLLOW) < 0 ||
        !same_snapshot_metadata(&snapshot->metadata, &descriptor_identity) ||
        !same_identity(&descriptor_identity, &pathname_identity) ||
        descriptors_equal(snapshot->identity_fd, snapshot->backup_fd,
                          snapshot->metadata.st_size) < 0) {
        errno = ESTALE;
        return -1;
    }
    struct stat after_compare;
    if (fstat(snapshot->identity_fd, &after_compare) < 0 ||
        !same_snapshot_metadata(&snapshot->metadata, &after_compare)) {
        errno = ESTALE;
        return -1;
    }
    return 0;
}

static int stage_managed_file(managed_snapshot *snapshot) {
    if (!snapshot->existed) {
        return 0;
    }
    if (verify_snapshot_unchanged(snapshot) < 0) {
        fprintf(stderr,
                "TR-300: the managed file changed after strict validation; "
                "rejecting PKG takeover.\n");
        return -1;
    }

    int placeholder = create_private_sibling(
        snapshot->directory->fd, snapshot->staged_name,
        sizeof(snapshot->staged_name));
    if (placeholder < 0) {
        report_errno("creating bound staging name for", snapshot->label);
        return -1;
    }
    if (close(placeholder) < 0) {
        int saved_errno = errno;
        (void)unlinkat(snapshot->directory->fd, snapshot->staged_name, 0);
        errno = saved_errno;
        return -1;
    }
    if (renameat(snapshot->directory->fd, snapshot->name,
                 snapshot->directory->fd, snapshot->staged_name) < 0) {
        int saved_errno = errno;
        (void)unlinkat(snapshot->directory->fd, snapshot->staged_name, 0);
        errno = saved_errno;
        report_errno("staging", snapshot->label);
        return -1;
    }
    snapshot->mutated = true;
    snapshot->staged = true;

    struct stat staged_descriptor;
    struct stat staged_pathname;
    struct stat staged_after_compare;
    if (fstat(snapshot->identity_fd, &staged_descriptor) < 0 ||
        fstatat(snapshot->directory->fd, snapshot->staged_name,
                &staged_pathname, AT_SYMLINK_NOFOLLOW) < 0 ||
        !same_post_rename_metadata(&snapshot->metadata, &staged_descriptor) ||
        !same_identity(&staged_descriptor, &staged_pathname) ||
        descriptors_equal(snapshot->identity_fd, snapshot->backup_fd,
                          snapshot->metadata.st_size) < 0 ||
        fstat(snapshot->identity_fd, &staged_after_compare) < 0 ||
        !same_snapshot_metadata(&staged_descriptor, &staged_after_compare) ||
        fsync(snapshot->directory->fd) < 0) {
        errno = ESTALE;
        fprintf(stderr,
                "TR-300: the staged managed file did not match the bound "
                "source identity; rejecting PKG takeover.\n");
        return -1;
    }
    return 0;
}

static int original_is_unlinked(const managed_snapshot *snapshot) {
    if (!snapshot->existed) {
        return 0;
    }
    struct stat current_identity;
    if (fstat(snapshot->identity_fd, &current_identity) < 0 ||
        !same_identity(&snapshot->metadata, &current_identity) ||
        current_identity.st_nlink != 0) {
        errno = ESTALE;
        return -1;
    }
    return 0;
}

static int discard_staged_file(managed_snapshot *snapshot) {
    if (!snapshot->staged) {
        return snapshot->existed ? original_is_unlinked(snapshot) : 0;
    }
    struct stat staged_identity;
    if (fstatat(snapshot->directory->fd, snapshot->staged_name,
                &staged_identity, AT_SYMLINK_NOFOLLOW) < 0 ||
        !same_identity(&snapshot->metadata, &staged_identity)) {
        errno = ESTALE;
        return -1;
    }
    if (unlinkat(snapshot->directory->fd, snapshot->staged_name, 0) < 0) {
        return -1;
    }
    snapshot->staged = false;
    snapshot->staged_name[0] = '\0';
    if (fsync(snapshot->directory->fd) < 0) {
        return -1;
    }
    return original_is_unlinked(snapshot);
}

static int discard_original_after_restore(managed_snapshot *snapshot) {
    if (!snapshot->mutated) {
        return 0;
    }
    if (snapshot->staged) {
        struct stat staged_identity;
        if (fstatat(snapshot->directory->fd, snapshot->staged_name,
                    &staged_identity, AT_SYMLINK_NOFOLLOW) < 0) {
            if (errno == ENOENT && original_is_unlinked(snapshot) == 0) {
                snapshot->staged = false;
                snapshot->staged_name[0] = '\0';
                return 0;
            }
            errno = ESTALE;
            return -1;
        }
        if (!same_identity(&snapshot->metadata, &staged_identity) ||
            unlinkat(snapshot->directory->fd, snapshot->staged_name, 0) < 0) {
            errno = ESTALE;
            return -1;
        }
        snapshot->staged = false;
        snapshot->staged_name[0] = '\0';
        if (fsync(snapshot->directory->fd) < 0) {
            return -1;
        }
    }
    return original_is_unlinked(snapshot);
}

#ifndef __APPLE__
static int restore_file_times(int fd, const struct stat *metadata) {
    struct timespec times[2];
    times[0] = metadata->st_atim;
    times[1] = metadata->st_mtim;
    return futimens(fd, times);
}

static int restore_file_ownership(int fd, const struct stat *metadata) {
    struct stat current;
    if (fstat(fd, &current) < 0) {
        return -1;
    }
    if (current.st_uid == metadata->st_uid && current.st_gid == metadata->st_gid) {
        return 0;
    }
    return fchown(fd, metadata->st_uid, metadata->st_gid);
}
#endif

static int restore_managed_file(managed_snapshot *snapshot) {
    if (!snapshot->mutated) {
        return 0;
    }
    if (snapshot->directory->fd < 0 || snapshot->backup_fd < 0) {
        errno = EBADF;
        return -1;
    }

    char temporary_name[NAME_MAX + 1U];
    int destination_fd = create_private_sibling(
        snapshot->directory->fd, temporary_name, sizeof(temporary_name));
    if (destination_fd < 0) {
        report_errno("creating atomic restore for", snapshot->label);
        return -1;
    }

    bool temporary_exists = true;
    int result = -1;
    if (copy_descriptor(snapshot->backup_fd, destination_fd,
                        snapshot->metadata.st_size,
                        snapshot->maximum_size) < 0 ||
        copy_descriptor_metadata(snapshot->backup_fd, destination_fd) < 0 ||
#ifndef __APPLE__
        restore_file_ownership(destination_fd, &snapshot->metadata) < 0 ||
        fchmod(destination_fd, snapshot->metadata.st_mode & 0777) < 0 ||
        restore_file_times(destination_fd, &snapshot->metadata) < 0 ||
#endif
        fsync(destination_fd) < 0) {
        report_errno("writing atomic restore for", snapshot->label);
        goto cleanup;
    }
    if (renameat(snapshot->directory->fd, temporary_name,
                 snapshot->directory->fd, snapshot->name) < 0) {
        report_errno("committing atomic restore for", snapshot->label);
        goto cleanup;
    }
    temporary_exists = false;
    struct stat destination_identity;
    struct stat restored_pathname;
    if (fstat(destination_fd, &destination_identity) < 0 ||
        fstatat(snapshot->directory->fd, snapshot->name, &restored_pathname,
                AT_SYMLINK_NOFOLLOW) < 0 ||
        !same_identity(&destination_identity, &restored_pathname)) {
        errno = ESTALE;
        report_errno("binding atomic restore for", snapshot->label);
        goto cleanup;
    }
    if (fsync(snapshot->directory->fd) < 0) {
        report_errno("syncing restored directory for", snapshot->label);
        goto cleanup;
    }
    if (close(destination_fd) < 0) {
        destination_fd = -1;
        report_errno("closing atomic restore for", snapshot->label);
        goto cleanup;
    }
    destination_fd = -1;
    result = 0;

cleanup:
    close_if_open(&destination_fd);
    if (temporary_exists) {
        (void)unlinkat(snapshot->directory->fd, temporary_name, 0);
    }
    return result;
}

static int verify_restored_file(const managed_snapshot *snapshot) {
    if (!snapshot->mutated) {
        return 0;
    }
    int restored_fd = openat(snapshot->directory->fd, snapshot->name,
                             O_RDONLY | O_NOFOLLOW | O_CLOEXEC);
    if (restored_fd < 0) {
        return -1;
    }
    struct stat restored;
    int result = 0;
    if (fstat(restored_fd, &restored) < 0 ||
        !S_ISREG(restored.st_mode) ||
        restored.st_uid != snapshot->metadata.st_uid ||
        restored.st_gid != snapshot->metadata.st_gid ||
        restored.st_mode != snapshot->metadata.st_mode ||
        restored.st_size != snapshot->metadata.st_size ||
#ifdef __APPLE__
        restored.st_flags != snapshot->metadata.st_flags ||
#endif
        descriptors_equal(restored_fd, snapshot->backup_fd,
                          snapshot->metadata.st_size) < 0) {
        errno = ESTALE;
        result = -1;
    }
    int saved_errno = errno;
    (void)close(restored_fd);
    errno = saved_errno;
    return result;
}

static int revalidate_directory(const char *home_path,
                                const struct stat *home_identity,
                                uid_t expected_uid,
                                const directory_binding *binding) {
    struct stat current_home_identity;
    int current_home_fd =
        open_absolute_directory_nofollow(home_path, &current_home_identity);
    if (current_home_fd < 0 ||
        !same_identity(home_identity, &current_home_identity)) {
        close_if_open(&current_home_fd);
        errno = ESTALE;
        return -1;
    }

    struct stat current_identity;
    int current_fd = open_relative_directory_nofollow(
        current_home_fd, binding->relative_path, expected_uid,
        &current_identity);
    close_if_open(&current_home_fd);

    if (!binding->existed) {
        if (current_fd < 0 && errno == ENOENT) {
            return 0;
        }
        close_if_open(&current_fd);
        errno = ESTALE;
        return -1;
    }
    if (current_fd < 0 || !same_identity(&binding->identity, &current_identity)) {
        close_if_open(&current_fd);
        errno = ESTALE;
        return -1;
    }
    close_if_open(&current_fd);
    return 0;
}

static int verify_name_absent(const managed_snapshot *snapshot) {
    if (!snapshot->directory->existed) {
        return 0;
    }
    struct stat ignored;
    if (fstatat(snapshot->directory->fd, snapshot->name, &ignored,
                AT_SYMLINK_NOFOLLOW) < 0 && errno == ENOENT) {
        return 0;
    }
    errno = EEXIST;
    return -1;
}

static void signal_handler(int signal_number) {
    caught_signal = signal_number;
    sig_atomic_t child = cleanup_pid;
    if (child > 0) {
        (void)kill((pid_t)child, signal_number);
    }
}

static int set_transaction_signal_handlers(void) {
    struct sigaction action;
    memset(&action, 0, sizeof(action));
    action.sa_handler = signal_handler;
    (void)sigemptyset(&action.sa_mask);
    return sigaction(SIGHUP, &action, NULL) == 0 &&
                   sigaction(SIGINT, &action, NULL) == 0 &&
                   sigaction(SIGTERM, &action, NULL) == 0
               ? 0
               : -1;
}

static int install_signal_handlers(struct sigaction *old_hup,
                                   struct sigaction *old_int,
                                   struct sigaction *old_term) {
    struct sigaction action;
    memset(&action, 0, sizeof(action));
    action.sa_handler = signal_handler;
    (void)sigemptyset(&action.sa_mask);
    if (sigaction(SIGHUP, &action, old_hup) < 0) {
        return -1;
    }
    if (sigaction(SIGINT, &action, old_int) < 0) {
        int saved_errno = errno;
        (void)sigaction(SIGHUP, old_hup, NULL);
        errno = saved_errno;
        return -1;
    }
    if (sigaction(SIGTERM, &action, old_term) < 0) {
        int saved_errno = errno;
        (void)sigaction(SIGHUP, old_hup, NULL);
        (void)sigaction(SIGINT, old_int, NULL);
        errno = saved_errno;
        return -1;
    }
    return 0;
}

static void restore_signal_handlers(const struct sigaction *old_hup,
                                    const struct sigaction *old_int,
                                    const struct sigaction *old_term) {
    (void)sigaction(SIGHUP, old_hup, NULL);
    (void)sigaction(SIGINT, old_int, NULL);
    (void)sigaction(SIGTERM, old_term, NULL);
}

static void transaction_signal_set(sigset_t *signals) {
    (void)sigemptyset(signals);
    (void)sigaddset(signals, SIGHUP);
    (void)sigaddset(signals, SIGINT);
    (void)sigaddset(signals, SIGTERM);
}

static int block_transaction_signals(sigset_t *old_mask) {
    sigset_t signals;
    transaction_signal_set(&signals);
    return sigprocmask(SIG_BLOCK, &signals, old_mask);
}

static int transaction_signal_pending(void) {
    sigset_t pending;
    if (sigpending(&pending) < 0) {
        return -1;
    }
    return sigismember(&pending, SIGHUP) || sigismember(&pending, SIGINT) ||
           sigismember(&pending, SIGTERM);
}

static int ignore_transaction_signals(void) {
    struct sigaction ignore;
    memset(&ignore, 0, sizeof(ignore));
    ignore.sa_handler = SIG_IGN;
    (void)sigemptyset(&ignore.sa_mask);
    if (sigaction(SIGHUP, &ignore, NULL) < 0 ||
        sigaction(SIGINT, &ignore, NULL) < 0 ||
        sigaction(SIGTERM, &ignore, NULL) < 0) {
        return -1;
    }
    return 0;
}

static int unblock_transaction_signals(const sigset_t *old_mask) {
    return sigprocmask(SIG_SETMASK, old_mask, NULL);
}

#ifdef TR300_ROLLBACK_TESTING
static int testing_after_first_discard(void) {
    const char *fixture_case = getenv("TR300_ROLLBACK_FIXTURE_CASE");
    if (fixture_case == NULL) {
        return 0;
    }
    if (strcmp(fixture_case, "partial-commit") == 0) {
        errno = EIO;
        return -1;
    }
    if (strcmp(fixture_case, "commit-signal") == 0) {
        /* The commit boundary has already blocked termination and installed
         * SIG_IGN. The invariant under test is that an actual signal cannot
         * interrupt the second discard; inspecting the transient pending mask
         * here makes the fixture platform-dependent and can re-arm the test
         * signal on the rollback path before it is discarded. */
        if (raise(SIGTERM) != 0) {
            errno = ECANCELED;
            return -1;
        }
    }
    return 0;
}
#endif

static int run_strict_cleanup(const char *user_home, uid_t expected_uid,
                              gid_t expected_gid, bool dry_run,
                              const char *cleanup_program) {
    trusted_executable executable;
    if (prepare_trusted_executable(cleanup_program, expected_uid, &executable) <
        0) {
        report_errno("binding trusted cleanup probe", cleanup_program);
        return -1;
    }

    pid_t child = fork();
    if (child < 0) {
        int saved_errno = errno;
        remove_trusted_executable(&executable);
        errno = saved_errno;
        return -1;
    }
    if (child == 0) {
#if !defined(TR300_ROLLBACK_TESTING)
        if (!dry_run) {
            _exit(126);
        }
#endif
#if !defined(TR300_ROLLBACK_TESTING) || \
    defined(TR300_ROLLBACK_PRIVILEGED_TESTING)
        if (setgroups(0, NULL) < 0 || setgid(expected_gid) < 0 ||
            setuid(expected_uid) < 0) {
            _exit(126);
        }
#else
        (void)expected_uid;
        (void)expected_gid;
#endif
        if (dry_run) {
            execl(executable.executable_path, executable.executable_path,
                  "migrate-cleanup", "--quiet", "--strict", "--dry-run",
                  "--cargo-copy", "--user-profile", user_home, (char *)NULL);
        }
#ifdef TR300_ROLLBACK_TESTING
        if (!dry_run) {
            execl(executable.executable_path, executable.executable_path,
                  "test-hook", user_home, (char *)NULL);
        }
#endif
        _exit(127);
    }

    cleanup_pid = (sig_atomic_t)child;
    int status = 0;
    for (;;) {
        pid_t waited = waitpid(child, &status, 0);
        if (waited == child) {
            break;
        }
        if (waited < 0 && errno == EINTR) {
            continue;
        }
        cleanup_pid = -1;
        int saved_errno = errno;
        remove_trusted_executable(&executable);
        errno = saved_errno;
        return -1;
    }
    cleanup_pid = -1;
    remove_trusted_executable(&executable);
    if (caught_signal != 0 || !WIFEXITED(status) || WEXITSTATUS(status) != 0) {
        errno = ECANCELED;
        return -1;
    }
    return 0;
}

static int inspect_transaction(const char *user_home, bool snapshot,
                               directory_binding *binary_directory,
                               directory_binding *receipt_directory,
                               managed_snapshot *binary,
                               managed_snapshot *receipt,
                               struct stat *home_identity,
                               uid_t *expected_uid, int *home_fd) {
    *home_fd = open_absolute_directory_nofollow(user_home, home_identity);
    if (*home_fd < 0) {
        report_errno("binding user home", user_home);
        return -1;
    }
    if (home_identity->st_uid == 0) {
        fprintf(stderr,
                "TR-300: the selected home is not owned by a normal user; "
                "rejecting PKG takeover.\n");
        errno = EPERM;
        return -1;
    }
    *expected_uid = home_identity->st_uid;

    if (bind_optional_directory(*home_fd, *expected_uid, binary_directory) < 0 ||
        bind_optional_directory(*home_fd, *expected_uid, receipt_directory) < 0) {
        return -1;
    }

    if (snapshot) {
        if (snapshot_managed_file(binary, *expected_uid) < 0 ||
            snapshot_managed_file(receipt, *expected_uid) < 0) {
            return -1;
        }
    } else {
        int source_fd = -1;
        if (inspect_managed_file(binary, *expected_uid, &source_fd) < 0) {
            return -1;
        }
        close_if_open(&source_fd);
        if (inspect_managed_file(receipt, *expected_uid, &source_fd) < 0) {
            return -1;
        }
        close_if_open(&source_fd);
    }
    return 0;
}

static int execute_transaction(const char *mode, const char *user_home,
                               const char *cleanup_program,
                               const char *preflight_state_path) {
    directory_binding binary_directory = {
        .fd = -1, .existed = false, .relative_path = ".cargo/bin"};
    directory_binding receipt_directory = {
        .fd = -1, .existed = false, .relative_path = ".config/tr300"};
    managed_snapshot binary = {.directory = &binary_directory,
                               .name = "tr300",
                               .label = "prior managed binary",
                               .maximum_size = TR300_BINARY_MAX_BYTES,
                               .executable = true,
                               .existed = false,
                               .backup_fd = -1,
                               .identity_fd = -1,
                               .mutated = false,
                               .staged = false,
                               .staged_name = {0}};
    managed_snapshot receipt = {.directory = &receipt_directory,
                                .name = "tr300-receipt.json",
                                 .label = "prior managed receipt",
                                 .maximum_size = TR300_RECEIPT_MAX_BYTES,
                                 .executable = false,
                                 .existed = false,
                                 .backup_fd = -1,
                                 .identity_fd = -1,
                                 .mutated = false,
                                 .staged = false,
                                 .staged_name = {0}};
    struct stat home_identity;
    uid_t expected_uid = 0;
    int home_fd = -1;
    int result = 1;

    bool should_snapshot = strcmp(mode, "run") == 0;
    if (inspect_transaction(user_home, should_snapshot, &binary_directory,
                            &receipt_directory, &binary, &receipt,
                            &home_identity, &expected_uid, &home_fd) < 0) {
        goto cleanup;
    }
    if (!should_snapshot) {
        if (cleanup_program == NULL ||
            run_strict_cleanup(user_home, expected_uid, home_identity.st_gid,
                               true, cleanup_program) == 0) {
            if (preflight_state_path == NULL ||
                (revalidate_directory(user_home, &home_identity, expected_uid,
                                      &binary_directory) == 0 &&
                 revalidate_directory(user_home, &home_identity, expected_uid,
                                      &receipt_directory) == 0 &&
                 write_preflight_state(preflight_state_path, &home_identity) ==
                     0)) {
                result = 0;
            } else {
                report_errno("persisting preinstall home identity",
                             preflight_state_path);
            }
        }
        goto cleanup;
    }

    if (preflight_state_path != NULL &&
        consume_preflight_state(preflight_state_path, &home_identity) < 0) {
        fprintf(stderr,
                "TR-300: the active user or home identity changed after "
                "preinstall; preserving managed state and failing PKG "
                "takeover.\n");
        goto cleanup;
    }

    struct sigaction old_hup;
    struct sigaction old_int;
    struct sigaction old_term;
    if (install_signal_handlers(&old_hup, &old_int, &old_term) < 0) {
        report_errno("installing rollback signal handlers", "");
        goto cleanup;
    }

    const char *run_cleanup_program = cleanup_program;
#ifndef TR300_ROLLBACK_TESTING
    run_cleanup_program = "/usr/local/bin/tr300";
#endif
    int preflight_result = run_strict_cleanup(
        user_home, expected_uid, home_identity.st_gid, true,
        run_cleanup_program);
    bool committed = false;
    bool signals_blocked = false;
    sigset_t old_signal_mask;
    if (preflight_result == 0 &&
        revalidate_directory(user_home, &home_identity, expected_uid,
                             &binary_directory) == 0 &&
        revalidate_directory(user_home, &home_identity, expected_uid,
                             &receipt_directory) == 0 &&
        verify_snapshot_unchanged(&binary) == 0 &&
        verify_snapshot_unchanged(&receipt) == 0 &&
        stage_managed_file(&binary) == 0 &&
        stage_managed_file(&receipt) == 0) {
        int hook_result = 0;
#ifdef TR300_ROLLBACK_TESTING
        hook_result = run_strict_cleanup(user_home, expected_uid,
                                         home_identity.st_gid, false,
                                         run_cleanup_program);
#endif
        if (hook_result == 0 && caught_signal == 0 &&
            revalidate_directory(user_home, &home_identity, expected_uid,
                                 &binary_directory) == 0 &&
            revalidate_directory(user_home, &home_identity, expected_uid,
                                 &receipt_directory) == 0 &&
            verify_name_absent(&binary) == 0 &&
            verify_name_absent(&receipt) == 0 &&
            block_transaction_signals(&old_signal_mask) == 0) {
            signals_blocked = true;
            if (caught_signal == 0 && transaction_signal_pending() == 0 &&
                revalidate_directory(user_home, &home_identity, expected_uid,
                                     &binary_directory) == 0 &&
                revalidate_directory(user_home, &home_identity, expected_uid,
                                     &receipt_directory) == 0 &&
                verify_name_absent(&binary) == 0 &&
                verify_name_absent(&receipt) == 0 &&
                ignore_transaction_signals() == 0) {
                int discard_result = discard_staged_file(&binary);
#ifdef TR300_ROLLBACK_TESTING
                if (discard_result == 0) {
                    discard_result = testing_after_first_discard();
                }
#endif
                if (discard_result == 0) {
                    discard_result = discard_staged_file(&receipt);
                }
                committed = discard_result == 0 &&
                            revalidate_directory(user_home, &home_identity,
                                                 expected_uid,
                                                 &binary_directory) == 0 &&
                            revalidate_directory(user_home, &home_identity,
                                                 expected_uid,
                                                 &receipt_directory) == 0 &&
                            verify_name_absent(&binary) == 0 &&
                            verify_name_absent(&receipt) == 0 &&
                            original_is_unlinked(&binary) == 0 &&
                            original_is_unlinked(&receipt) == 0;
            }
        }
    }

    bool mutation_started = binary.mutated || receipt.mutated;
    if (!committed && mutation_started) {
        int binary_restore = restore_managed_file(&binary);
        int receipt_restore = restore_managed_file(&receipt);
        int binary_discard = binary_restore == 0
                                 ? discard_original_after_restore(&binary)
                                 : -1;
        int receipt_discard = receipt_restore == 0
                                  ? discard_original_after_restore(&receipt)
                                  : -1;
        int binary_directory_current = revalidate_directory(
            user_home, &home_identity, expected_uid, &binary_directory);
        int receipt_directory_current = revalidate_directory(
            user_home, &home_identity, expected_uid, &receipt_directory);
        int binary_verified = verify_restored_file(&binary);
        int receipt_verified = verify_restored_file(&receipt);
        if (binary_restore < 0 || receipt_restore < 0 || binary_discard < 0 ||
            receipt_discard < 0 || binary_directory_current < 0 ||
            receipt_directory_current < 0 || binary_verified < 0 ||
            receipt_verified < 0) {
            fprintf(stderr,
                    "TR-300: strict cleanup did not commit and descriptor-bound "
                    "rollback was incomplete; the package must fail closed.\n");
        } else {
            fprintf(stderr,
                    "TR-300: strict cleanup did not commit; restored the prior "
                    "managed state through bound directories.\n");
        }
        result = 1;
    } else if (committed) {
        result = 0;
    } else {
        fprintf(stderr,
                "TR-300: managed ownership changed during strict validation; "
                "the package must fail before takeover.\n");
        result = 1;
    }

    if (signals_blocked) {
        if (!committed) {
            (void)set_transaction_signal_handlers();
        }
        if (committed) {
            caught_signal = 0;
        }
        if (unblock_transaction_signals(&old_signal_mask) < 0) {
            report_errno("restoring rollback signal mask", "");
            if (!committed) {
                result = 1;
            }
        }
    }
    if (!committed) {
        restore_signal_handlers(&old_hup, &old_int, &old_term);
    }

cleanup:
    close_if_open(&binary.backup_fd);
    close_if_open(&receipt.backup_fd);
    close_if_open(&binary.identity_fd);
    close_if_open(&receipt.identity_fd);
    close_if_open(&binary_directory.fd);
    close_if_open(&receipt_directory.fd);
    close_if_open(&home_fd);
    return result;
}

static void usage(const char *program) {
#ifdef TR300_ROLLBACK_TESTING
    fprintf(stderr, "usage: %s check <absolute-user-home> [cleanup-fixture "
                    "[preflight-state]] | run <absolute-user-home> "
                    "<cleanup-fixture> [preflight-state]\n",
            program);
#else
    fprintf(stderr,
            "usage: %s check <absolute-user-home> <embedded-probe> "
            "<preflight-state> | run <absolute-user-home> "
            "<preflight-state>\n",
            program);
#endif
}

int main(int argc, char **argv) {
#if !defined(TR300_ROLLBACK_TESTING) || \
    defined(TR300_ROLLBACK_PRIVILEGED_TESTING)
    if (geteuid() != 0) {
        fprintf(stderr,
                "TR-300: the PKG rollback helper must run inside Apple's root "
                "installer transaction.\n");
        return 77;
    }
#endif
    (void)umask(077);
#ifdef __APPLE__
    if (unsetenv(COPYFILE_DISABLE_VAR) < 0) {
        report_errno("clearing copyfile metadata override", "");
        return 70;
    }
#endif

    if (argc < 3 || (strcmp(argv[1], "check") != 0 &&
                     strcmp(argv[1], "run") != 0)) {
        usage(argv[0]);
        return 64;
    }
#ifdef TR300_ROLLBACK_TESTING
    if ((strcmp(argv[1], "check") == 0 && argc != 3 && argc != 4 &&
         argc != 5) ||
        (strcmp(argv[1], "run") == 0 && argc != 4 && argc != 5)) {
        usage(argv[0]);
        return 64;
    }
    const char *cleanup_program = argc >= 4 ? argv[3] : NULL;
    const char *preflight_state_path = argc == 5 ? argv[4] : NULL;
#else
    if ((strcmp(argv[1], "check") == 0 && argc != 5) ||
        (strcmp(argv[1], "run") == 0 && argc != 4)) {
        usage(argv[0]);
        return 64;
    }
    const char *cleanup_program =
        strcmp(argv[1], "check") == 0 ? argv[3] : NULL;
    const char *preflight_state_path =
        strcmp(argv[1], "check") == 0 ? argv[4] : argv[3];
#endif

    int result = execute_transaction(argv[1], argv[2], cleanup_program,
                                     preflight_state_path);
    if (caught_signal != 0) {
        return 128 + caught_signal;
    }
    return result;
}
