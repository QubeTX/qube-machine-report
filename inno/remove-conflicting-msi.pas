{ Remove the same-edition MSI before an explicit Inno install. This runs from
  PrepareToInstall, before Inno writes any files, so the fresh installer is the
  user's latest channel choice without leaving a second Windows Installer
  registration behind. The calling .iss defines ConflictingMsiUpgradeCode. }

const
  ErrorSuccess = 0;
  ErrorNoMoreItems = 259;
  ErrorUnknownProduct = 1605;
  ErrorSuccessRebootInitiated = 1641;
  ErrorSuccessRebootRequired = 3010;

function MsiEnumRelatedProducts(
  UpgradeCode: string; Reserved: Cardinal; ProductIndex: Cardinal;
  var ProductCode: string): Cardinal;
  external 'MsiEnumRelatedProductsW@msi.dll stdcall';

function PrepareToInstall(var NeedsRestart: Boolean): String;
var
  ProductCode: string;
  EnumResult: Cardinal;
  ExitCode: Integer;
  Args: string;
  RemovedProducts: Integer;
begin
  Result := '';
  RemovedProducts := 0;
  while RemovedProducts < 32 do
  begin
    SetLength(ProductCode, 39);
    { Always enumerate index zero: removing a product compacts the related-product
      set, so another historical registration (if present) becomes index zero. }
    EnumResult := MsiEnumRelatedProducts(
      '{#ConflictingMsiUpgradeCode}', 0, 0, ProductCode);
    if EnumResult = ErrorNoMoreItems then
      exit;
    if EnumResult <> ErrorSuccess then
    begin
      Result := 'Could not safely inspect the existing MSI installation (Windows Installer error ' +
        IntToStr(EnumResult) + '). The existing install was left unchanged.';
      exit;
    end;

    ProductCode := Copy(ProductCode, 1, 38);
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
    RemovedProducts := RemovedProducts + 1;
  end;

  Result := 'More than 32 related MSI registrations were found. ' +
    'The installer stopped rather than guessing which additional products are safe to remove.';
end;
