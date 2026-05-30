import { NextResponse } from 'next/server';

/**
 * Example API route demonstrating CSRF protection
 * 
 * GET requests receive a CSRF token automatically
 * POST/PUT/DELETE/PATCH requests require valid CSRF token
 */

export async function GET() {
  return NextResponse.json({
    message: 'CSRF token has been set in cookie and header',
    timestamp: new Date().toISOString()
  });
}

export async function POST(request: Request) {
  // CSRF validation happens automatically in middleware
  // If we reach here, the token was valid
  
  const body = await request.json().catch(() => ({}));
  
  return NextResponse.json({
    message: 'POST request successful - CSRF token validated',
    data: body,
    timestamp: new Date().toISOString()
  });
}

export async function DELETE() {
  // CSRF validation happens automatically in middleware
  
  return NextResponse.json({
    message: 'DELETE request successful - CSRF token validated',
    timestamp: new Date().toISOString()
  });
}
