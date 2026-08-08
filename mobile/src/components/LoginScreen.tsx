import React from 'react';
import {
  ActivityIndicator,
  Platform,
  Pressable,
  StyleSheet,
  Text,
  TextInput,
  View,
} from 'react-native';
import { SafeAreaView } from 'react-native-safe-area-context';

import { useLoginScreen } from '@hooks/useLoginScreen';

const ACCENT_COLOR = Platform.OS === 'ios' ? '#007AFF' : '#1976D2';

/** Props for {@link LoginScreen}. */
export interface LoginScreenProps {
  /** Invoked after a successful authentication (e.g. to navigate onward). */
  onLoginSuccess?: () => void;
}

/**
 * Unauthenticated login screen with email/username and password fields,
 * inline field validation, a global error banner, a password visibility
 * toggle, and a loading state. All form logic lives in `useLoginScreen`.
 *
 * @param props - {@link LoginScreenProps}.
 */
export const LoginScreen: React.FC<LoginScreenProps> = ({ onLoginSuccess }) => {
  const {
    identifier,
    password,
    showPassword,
    fieldErrors,
    formError,
    loading,
    setIdentifier,
    setPassword,
    toggleShowPassword,
    handleSubmit,
    canUseBiometrics,
    biometricLabel,
    handleBiometricLogin,
  } = useLoginScreen({ onLoginSuccess });

  return (
    <SafeAreaView style={styles.container}>
      <View style={styles.content}>
        <View style={styles.header}>
          <Text style={styles.title} accessibilityRole="header">
            PayRaider
          </Text>
          <Text style={styles.subtitle}>Sign in to continue</Text>
        </View>

        {formError ? (
          <View
            style={styles.errorBanner}
            accessible
            accessibilityRole="alert"
            accessibilityLabel={formError}
          >
            <Text style={styles.errorBannerText}>{formError}</Text>
          </View>
        ) : null}

        <View style={styles.field}>
          <Text style={styles.label}>Email or username</Text>
          <TextInput
            value={identifier}
            onChangeText={setIdentifier}
            placeholder="you@example.com"
            placeholderTextColor="#9CA3AF"
            autoCapitalize="none"
            autoCorrect={false}
            keyboardType="email-address"
            textContentType="username"
            editable={!loading}
            style={[styles.input, !!fieldErrors.identifier && styles.inputError]}
            accessibilityLabel="Email or username"
            accessibilityHint="Enter the email address or username for your account"
          />
          {fieldErrors.identifier ? (
            <Text
              style={styles.fieldError}
              accessibilityRole="alert"
              accessibilityLabel={fieldErrors.identifier}
            >
              {fieldErrors.identifier}
            </Text>
          ) : null}
        </View>

        <View style={styles.field}>
          <Text style={styles.label}>Password</Text>
          <View style={styles.passwordRow}>
            <TextInput
              value={password}
              onChangeText={setPassword}
              placeholder="Your password"
              placeholderTextColor="#9CA3AF"
              autoCapitalize="none"
              autoCorrect={false}
              secureTextEntry={!showPassword}
              textContentType="password"
              editable={!loading}
              style={[
                styles.input,
                styles.passwordInput,
                !!fieldErrors.password && styles.inputError,
              ]}
              accessibilityLabel="Password"
              accessibilityHint="Enter your account password"
            />
            <Pressable
              onPress={toggleShowPassword}
              style={styles.toggleButton}
              accessibilityRole="button"
              accessibilityLabel={showPassword ? 'Hide password' : 'Show password'}
              accessibilityHint="Toggles whether the password is visible"
            >
              <Text style={styles.toggleButtonText}>
                {showPassword ? 'Hide' : 'Show'}
              </Text>
            </Pressable>
          </View>
          {fieldErrors.password ? (
            <Text
              style={styles.fieldError}
              accessibilityRole="alert"
              accessibilityLabel={fieldErrors.password}
            >
              {fieldErrors.password}
            </Text>
          ) : null}
        </View>

        <Pressable
          onPress={() => {
            void handleSubmit();
          }}
          disabled={loading}
          style={({ pressed }) => [
            styles.submitButton,
            pressed && styles.submitButtonPressed,
            loading && styles.submitButtonDisabled,
          ]}
          accessibilityRole="button"
          accessibilityState={{ disabled: loading, busy: loading }}
          accessibilityLabel="Sign in"
          accessibilityHint="Submits your credentials to sign in"
        >
          {loading ? (
            <ActivityIndicator color="#FFFFFF" />
          ) : (
            <Text style={styles.submitButtonText}>Sign In</Text>
          )}
        </Pressable>

        {canUseBiometrics ? (
          <Pressable
            onPress={() => {
              void handleBiometricLogin();
            }}
            disabled={loading}
            style={({ pressed }) => [
              styles.biometricButton,
              pressed && styles.biometricButtonPressed,
            ]}
            accessibilityRole="button"
            accessibilityLabel={`Sign in with ${biometricLabel}`}
            accessibilityHint={`Uses ${biometricLabel} to sign in with your saved account`}
          >
            <Text style={styles.biometricButtonText}>
              Sign in with {biometricLabel}
            </Text>
          </Pressable>
        ) : null}
      </View>
    </SafeAreaView>
  );
};

const styles = StyleSheet.create({
  container: {
    flex: 1,
    backgroundColor: Platform.OS === 'ios' ? '#F2F2F7' : '#FAFAFA',
  },
  content: {
    flex: 1,
    justifyContent: 'center',
    paddingHorizontal: 24,
  },
  header: {
    marginBottom: 32,
    alignItems: 'center',
  },
  title: {
    fontSize: Platform.OS === 'ios' ? 32 : 28,
    fontWeight: '700',
    color: '#111827',
  },
  subtitle: {
    marginTop: 8,
    fontSize: 16,
    color: '#6B7280',
  },
  biometricButton: {
    minHeight: 44,
    marginTop: 16,
    borderRadius: 8,
    borderWidth: 1,
    borderColor: ACCENT_COLOR,
    alignItems: 'center',
    justifyContent: 'center',
    paddingVertical: 12,
  },
  biometricButtonPressed: {
    opacity: 0.7,
  },
  biometricButtonText: {
    color: ACCENT_COLOR,
    fontSize: 16,
    fontWeight: '600',
  },
  errorBanner: {
    backgroundColor: '#FEE2E2',
    borderColor: '#EF4444',
    borderWidth: 1,
    borderRadius: 8,
    padding: 12,
    marginBottom: 16,
  },
  errorBannerText: {
    color: '#991B1B',
    fontSize: 14,
  },
  field: {
    marginBottom: 16,
  },
  label: {
    fontSize: 14,
    fontWeight: '600',
    color: '#374151',
    marginBottom: 6,
  },
  input: {
    backgroundColor: '#FFFFFF',
    borderWidth: 1,
    borderColor: '#D1D5DB',
    borderRadius: 8,
    paddingHorizontal: 12,
    paddingVertical: Platform.OS === 'ios' ? 14 : 10,
    fontSize: 16,
    color: '#111827',
    minHeight: 44,
  },
  inputError: {
    borderColor: '#EF4444',
  },
  passwordRow: {
    flexDirection: 'row',
    alignItems: 'center',
  },
  passwordInput: {
    flex: 1,
  },
  toggleButton: {
    marginLeft: 8,
    paddingHorizontal: 12,
    minHeight: 44,
    justifyContent: 'center',
  },
  toggleButtonText: {
    color: ACCENT_COLOR,
    fontSize: 14,
    fontWeight: '600',
  },
  fieldError: {
    marginTop: 6,
    fontSize: 13,
    color: '#B91C1C',
  },
  submitButton: {
    marginTop: 8,
    backgroundColor: ACCENT_COLOR,
    borderRadius: 8,
    paddingVertical: 14,
    alignItems: 'center',
    justifyContent: 'center',
    minHeight: 48,
  },
  submitButtonPressed: {
    opacity: 0.85,
  },
  submitButtonDisabled: {
    opacity: 0.6,
  },
  submitButtonText: {
    color: '#FFFFFF',
    fontSize: 16,
    fontWeight: '700',
  },
});
