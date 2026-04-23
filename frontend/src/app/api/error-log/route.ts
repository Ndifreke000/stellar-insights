import { NextResponse } from 'next/server';
import * as Sentry from '@sentry/nextjs';

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
    console.error('[ErrorLog API] Frontend error captured:', {
      message: errorMessage,
      stack,
      metadata,
    });
  }

  return NextResponse.json({ success: true });
}
