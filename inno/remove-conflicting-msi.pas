{ Remove same-edition MSI registrations before an explicit Inno install. This
  runs from PrepareToInstall, before Inno writes any files, so the fresh
  installer is the user's latest channel choice without leaving a second
  Windows Installer registration behind.

  Do not call MsiEnumRelatedProductsW directly from Pascal Script here. Hosted
  Inno Setup 6.7.1 proved that the output-buffer ABI can access-violate Setup
  even when the declaration looks correct. Instead, use Inno's supported
  registry helpers and require exact native Add/Remove Programs evidence:
  scope, display name, publisher, WindowsInstaller=1, and a GUID product key.
  The calling .iss defines ConflictingMsiDisplayName and
  ConflictingMsiPublisher; Global also defines ConflictingMsiPerMachine. }

const
  ErrorSuccess = 0;
  ErrorUnknownProduct = 1605;
  ErrorSuccessRebootInitiated = 1641;
  ErrorSuccessRebootRequired = 3010;
  UninstallKey = 'Software\Microsoft\Windows\CurrentVersion\Uninstall';
  MaxConflictingMsiProducts = 32;

function IsHexDigit(Value: Char): Boolean;
begin
  Result := ((Value >= '0') and (Value <= '9')) or
    ((Value >= 'A') and (Value <= 'F')) or
    ((Value >= 'a') and (Value <= 'f'));
end;

function IsProductCode(Value: String): Boolean;
var
  Index: Integer;
begin
  Result := False;
  if (Length(Value) <> 38) or (Value[1] <> '{') or (Value[38] <> '}') then
    exit;

  for Index := 2 to 37 do
  begin
    if (Index = 10) or (Index = 15) or (Index = 20) or (Index = 25) then
    begin
      if Value[Index] <> '-' then
        exit;
    end
    else if not IsHexDigit(Value[Index]) then
      exit;
  end;
  Result := True;
end;

procedure AddMatchingMsiProducts(RootKey: Integer; var ProductCodes: TArrayOfString);
var
  Subkeys: TArrayOfString;
  Key: String;
  DisplayName: String;
  Publisher: String;
  WindowsInstaller: Cardinal;
  Index: Integer;
  Count: Integer;
begin
  { A missing uninstall key simply means this scope has no registered product. }
  if not RegGetSubkeyNames(RootKey, UninstallKey, Subkeys) then
    exit;

  for Index := 0 to GetArrayLength(Subkeys) - 1 do
  begin
    Key := UninstallKey + '\' + Subkeys[Index];
    if IsProductCode(Subkeys[Index]) and
       RegQueryStringValue(RootKey, Key, 'DisplayName', DisplayName) and
       (DisplayName = '{#ConflictingMsiDisplayName}') and
       RegQueryStringValue(RootKey, Key, 'Publisher', Publisher) and
       (Publisher = '{#ConflictingMsiPublisher}') and
       RegQueryDWordValue(RootKey, Key, 'WindowsInstaller', WindowsInstaller) and
       (WindowsInstaller = 1) then
    begin
      Count := GetArrayLength(ProductCodes);
      SetArrayLength(ProductCodes, Count + 1);
      ProductCodes[Count] := Subkeys[Index];
    end;
  end;
end;

function PrepareToInstall(var NeedsRestart: Boolean): String;
var
  ProductCodes: TArrayOfString;
  ProductCode: String;
  ExitCode: Integer;
  Args: String;
  Index: Integer;
begin
  Result := '';

#ifdef ConflictingMsiPerMachine
  AddMatchingMsiProducts(HKEY_LOCAL_MACHINE_64, ProductCodes);
#else
  AddMatchingMsiProducts(HKEY_CURRENT_USER, ProductCodes);
#endif

  if GetArrayLength(ProductCodes) > MaxConflictingMsiProducts then
  begin
    Result := 'More than 32 matching MSI registrations were found. ' +
      'The installer stopped before changing anything rather than guessing.';
    exit;
  end;

  for Index := 0 to GetArrayLength(ProductCodes) - 1 do
  begin
    ProductCode := ProductCodes[Index];
    Args := '/x "' + ProductCode + '" /qn /norestart';
    Log('Removing same-edition MSI before changing the install channel: ' + ProductCode);
    if not Exec(ExpandConstant('{sys}\msiexec.exe'), Args, '', SW_HIDE,
        ewWaitUntilTerminated, ExitCode) then
    begin
      Result := 'Could not launch Windows Installer to remove the previous MSI. ' +
        'The existing installation was left unchanged.';
      exit;
    end;

    if (ExitCode = ErrorSuccessRebootInitiated) or
       (ExitCode = ErrorSuccessRebootRequired) then
    begin
      NeedsRestart := True;
      Result := 'Windows must restart to finish removing the previous MSI. ' +
        'Restart, then run this installer again.';
      exit;
    end;
    if (ExitCode <> ErrorSuccess) and (ExitCode <> ErrorUnknownProduct) then
    begin
      Result := 'The previous MSI could not be removed safely (exit ' +
        IntToStr(ExitCode) + '). It remains installed; this installer did not continue.';
      exit;
    end;
  end;
end;
