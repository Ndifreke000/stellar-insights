export interface ValidationResult {
  isValid: boolean;
  error: string;
}

export const validateUrl = (value: string): ValidationResult => {
  if (!value.trim()) {
    return { isValid: false, error: 'Transfer server URL is required' };
  }
  try {
    new URL(value.trim());
    if (!value.trim().startsWith('https://')) {
      return { isValid: false, error: 'URL must use HTTPS' };
    }
    return { isValid: true, error: '' };
  } catch {
    return { isValid: false, error: 'Enter a valid URL (e.g., https://api.example.com/sep24)' };
  }
};

export const validateAmount = (value: string): ValidationResult => {
  const num = parseFloat(value);
  if (!value.trim() || isNaN(num) || num <= 0) {
    return { isValid: false, error: 'Amount must be a positive number' };
  }
  if (num > 1e12) {
    return { isValid: false, error: 'Amount too large' };
  }
  const parts = value.split('.');
  if (parts.length > 2 || (parts[1] && parts[1].length > 7)) {
    return { isValid: false, error: 'Max 7 decimal places' };
  }
  return { isValid: true, error: '' };
};

export const validateStellarAccount = (value: string): ValidationResult => {
  if (!value.trim()) {
    return { isValid: true, error: '' }; // optional
  }
  const regex = /^G[A-Z0-9]{55}$/;
  if (!regex.test(value.trim())) {
    return { isValid: false, error: 'Invalid Stellar account (must be 56 chars starting with G)' };
  }
  return { isValid: true, error: '' };
};

export const validateReceiverId = (value: string): ValidationResult => {
  if (!value.trim()) {
    return { isValid: false, error: 'Receiver ID is required' };
  }
  if (value.trim().length < 3 || value.trim().length > 256) {
    return { isValid: false, error: 'Receiver ID must be 3-256 chars' };
  }
  return { isValid: true, error: '' };
};

export const validateAssetCode = (value: string): ValidationResult => {
  if (!value.trim()) {
    return { isValid: true, error: '' }; // optional
  }
  const regex = /^[A-Z0-9]+(:[A-Z0-9]+)?$/;
  if (!regex.test(value.trim()) || value.trim().length > 64) {
    return { isValid: false, error: 'Invalid asset (e.g., USDC or USDC:GBEZET...)' };
  }
  return { isValid: true, error: '' };
};

export const validateJwt = (value: string): ValidationResult => {
  if (!value.trim()) {
    return { isValid: true, error: '' }; // optional
  }
  if (value.trim().length < 10) {
    return { isValid: false, error: 'JWT too short' };
  }
  return { isValid: true, error: '' };
};

export const getFieldErrorId = (field: string) => `error-${field}`;
