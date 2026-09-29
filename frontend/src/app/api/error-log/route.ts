import { NextResponse } from 'next/server';
import * as Sentry from '@sentry/nextjs';

// Server-side logging for API routes
function logError(message: string, metadata?: Record<string, unknown>) {
  if (process.env.NODE_ENV === 'development') {
    console.error(message, metadata);
  } else {
    // In production, rely on Sentry or structured logging service
    Sentry.captureMessage(message, 'error');
  }
}

export async function POST(request: Request) {
  const body = await request.json().catch(() => null);

  if (!body || typeof body !== 'object') {
    return NextResponse.json({ error: 'Invalid error payload' }, { status: 400 });
  }

  const { message, stack, metadata } = body as {
    message?: string;
    stack?: string;
    metadata?: Record<string, unknown>;
  };

  const errorMessage = message || 'Unknown frontend error';

  if (process.env.SENTRY_DSN) {
    Sentry.captureException(new Error(errorMessage), {
      tags: {
        logger: 'frontend',
      },
      extra: {
        stack,
        metadata,
      },
    });
  } else {
    logError('[ErrorLog API] Frontend error captured:', {
      message: errorMessage,
      stack,
      metadata,
    });
  }

  return NextResponse.json({ success: true });
}
