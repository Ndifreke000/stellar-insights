import React from 'react';
import { View, Text, StyleSheet, Pressable } from 'react-native';
import { useNavigation, useRoute, RouteProp } from '@react-navigation/native';
import { RootStackParamList } from '../navigation/RootNavigator';
import { NativeStackNavigationProp } from '@react-navigation/native-stack';

type ErrorScreenRouteProp = RouteProp<RootStackParamList, 'Error'>;
type NavigationProp = NativeStackNavigationProp<RootStackParamList, 'Error'>;

export function ErrorScreen() {
  const route = useRoute<ErrorScreenRouteProp>();
  const navigation = useNavigation<NavigationProp>();
  const message = route.params?.message ?? 'An invalid deep link was provided.';

  return (
    <View style={styles.container}>
      <View style={styles.card}>
        <Text style={styles.title}>Invalid Link</Text>
        <Text style={styles.message}>{message}</Text>
        <Pressable
          onPress={() => navigation.navigate('Main')}
          style={styles.button}
          accessibilityRole="button"
          accessibilityLabel="Go to main dashboard"
        >
          <Text style={styles.buttonText}>Go to Dashboard</Text>
        </Pressable>
      </View>
    </View>
  );
}

const styles = StyleSheet.create({
  container: {
    flex: 1,
    backgroundColor: '#FAFAFA',
    justifyContent: 'center',
    alignItems: 'center',
    padding: 24,
  },
  card: {
    backgroundColor: '#FFFFFF',
    borderRadius: 12,
    padding: 24,
    width: '100%',
    maxWidth: 340,
    alignItems: 'center',
    borderWidth: 1,
    borderColor: '#FCA5A5',
    elevation: 2,
    shadowColor: '#000',
    shadowOpacity: 0.1,
    shadowRadius: 4,
    shadowOffset: { width: 0, height: 2 },
  },
  title: {
    fontSize: 20,
    fontWeight: '700',
    color: '#DC2626',
    marginBottom: 12,
  },
  message: {
    fontSize: 14,
    color: '#4B5563',
    textAlign: 'center',
    marginBottom: 20,
    lineHeight: 20,
  },
  button: {
    backgroundColor: '#DC2626',
    borderRadius: 8,
    paddingVertical: 12,
    paddingHorizontal: 24,
    alignItems: 'center',
    justifyContent: 'center',
    minHeight: 44,
  },
  buttonText: {
    color: '#FFFFFF',
    fontSize: 15,
    fontWeight: '600',
  },
});
