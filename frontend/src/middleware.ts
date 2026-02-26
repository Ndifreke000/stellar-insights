import createMiddleware from "next-intl/middleware";
import { routing } from "./i18n/routing";
import { NextResponse } from 'next/server';
import type { NextRequest } from 'next/server';
import { generateCsrfToken, validateCsrfToken } from './lib/csrf';

const intlMiddleware = createMiddleware(routing);

export default function middleware(request: NextRequest) {
  // Handle API routes with CSRF protection
  if (request.nextUrl.pathname.startsWith('/api/')) {
    return handleApiRequest(request);
  }
  
  // Handle regular routes with i18n
  return intlMiddleware(request);
}

function handleApiRequest(request: NextRequest) {
  const response = NextResponse.next();
  
  // Generate and set CSRF token for GET requests
  if (request.method === 'GET') {
    const csrfToken = generateCsrfToken();
    response.cookies.set('csrf-token', csrfToken, {
      httpOnly: true,
      secure: process.env.NODE_ENV === 'production',
      sameSite: 'strict',
      path: '/',
      maxAge: 60 * 60 * 24 // 24 hours
    });
    response.headers.set('X-CSRF-Token', csrfToken);
    return response;
  }
  
  // Validate CSRF token for state-changing requests
  if (['POST', 'PUT', 'DELETE', 'PATCH'].includes(request.method)) {
    const cookieToken = request.cookies.get('csrf-token')?.value;
    const headerToken = request.headers.get('X-CSRF-Token');
    
    if (!validateCsrfToken(cookieToken, headerToken)) {
      return NextResponse.json(
        { 
          error: 'Invalid CSRF token',
          message: 'CSRF token validation failed. Please refresh the page and try again.'
        },
        { status: 403 }
      );
    }
  }
  
  return response;
}

export const config = {
  matcher: [
    "/((?!_next|_vercel|.*\\..*).*)", // All routes except static files
  ],
};
