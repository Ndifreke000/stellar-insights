import { NextResponse } from 'next/server';
import * as Sentry from '@sentry/nextjs';

/**
 * Test error endpoint to verify Sentry error tracking with source maps.
 * Can be triggered via GET /api/sentry-test
 */
export async function GET() {
  const testError = new Error('PayRaider Sentry Verification: Test frontend error tracking with source maps');
  
  // Capture exception in Sentry with release correlation
  const eventId = Sentry.captureException(testError, {
    tags: {
      type: 'sentry-verification',
      environment: process.env.NODE_ENV || 'development',
    },
    extra: {
      timestamp: new Date().toISOString(),
      release: process.env.NEXT_PUBLIC_APP_VERSION || 'development',
    },
  });

  return NextResponse.json({
    status: 'ok',
    message: 'Sentry test error generated and captured',
    eventId,
    release: process.env.NEXT_PUBLIC_APP_VERSION || 'development',
  });
}
