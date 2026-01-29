// Prisma client setup - only used if Prisma is properly configured
let PrismaClient: unknown;
let prisma: unknown;

try {
  // Try to import PrismaClient
  // eslint-disable-next-line @typescript-eslint/no-require-imports
  const prismaModule = require('@prisma/client');
  PrismaClient = prismaModule.PrismaClient;
  
  const globalForPrisma = globalThis as unknown as {
    prisma: unknown | undefined
  }

  prisma = globalForPrisma.prisma ?? new (PrismaClient as new () => unknown)()

  if (process.env.NODE_ENV !== 'production') globalForPrisma.prisma = prisma
} catch {
  // Prisma client not available, create a mock
  console.warn('Prisma client not available, using mock');
  prisma = null;
}

export { prisma };
