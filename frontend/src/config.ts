const apiUrl = process.env.NEXT_PUBLIC_API_URL || "http://127.0.0.1:8080/api";

export const config = {
  apiUrl,
} as const;
