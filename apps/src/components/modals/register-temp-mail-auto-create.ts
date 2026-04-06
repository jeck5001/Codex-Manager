const TEMP_MAIL_TYPE = "temp_mail";

export function canShowTempMailAutoCreateToggle(registerServiceType: string): boolean {
  return registerServiceType === TEMP_MAIL_TYPE;
}

export function shouldDisableTempMailServicePicker(
  registerServiceType: string,
  autoCreateTempMailService: boolean,
): boolean {
  return canShowTempMailAutoCreateToggle(registerServiceType) && autoCreateTempMailService;
}

export function deriveTempMailAutoCreateSubmitFlag(
  registerServiceType: string,
  autoCreateTempMailService: boolean,
): boolean {
  return canShowTempMailAutoCreateToggle(registerServiceType) && autoCreateTempMailService;
}

export function shouldBypassTempMailServicePickerOnSubmit(
  registerServiceType: string,
  autoCreateTempMailService: boolean,
): boolean {
  return deriveTempMailAutoCreateSubmitFlag(registerServiceType, autoCreateTempMailService);
}
