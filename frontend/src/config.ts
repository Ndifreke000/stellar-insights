const apiUrl = process.env.NEXT_PUBLIC_API_URL;

if (!apiUrl) {
  throw new Error("NEXT_PUBLIC_API_URL environment variable is required");
}

export const config = {
  apiUrl,
} as const;
