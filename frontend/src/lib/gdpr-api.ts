// GDPR API Client - Frontend API calls for GDPR compliance features

const API_BASE_URL = process.env.NEXT_PUBLIC_API_URL || "http://localhost:8080";

// Types matching the backend models

export interface ConsentResponse {
  consent_type: string;
  consent_given: boolean;
  consent_version: string;
  granted_at: string | null;
  revoked_at: string | null;
}

export interface UpdateConsentRequest {
  consent_type: string;
  consent_given: boolean;
  consent_version?: string;
}

export interface BatchUpdateConsentsRequest {
  consents: UpdateConsentRequest[];
}

export interface ExportRequestResponse {
  id: string;
  status: "pending" | "processing" | "completed" | "expired" | "failed";
  requested_at: string;
  expires_at: string | null;
  download_url: string | null;
}

export interface CreateExportRequest {
  data_types: string[];
  export_format?: string;
}

export interface DeletionRequestResponse {
  id: string;
  status:
    | "pending"
    | "scheduled"
    | "processing"
    | "completed"
    | "cancelled"
    | "failed";
  requested_at: string;
  scheduled_deletion_at: string | null;
  confirmation_required: boolean;
  confirmation_token: string | null;
}

export interface CreateDeletionRequest {
  reason?: string;
  delete_all_data?: boolean;
  data_types?: string[];
}

export interface ConfirmDeletionRequest {
  confirmation_token: string;
}

export interface DataTypeInfo {
  id: string;
  name: string;
  description: string;
  category: string;
}

export interface ExportableDataTypes {
  types: DataTypeInfo[];
}

export interface GdprSummary {
  user_id: string;
  consents: ConsentResponse[];
  pending_export_requests: number;
  pending_deletion_requests: number;
  data_processing_activities_count: number;
}

// API functions

async function fetchWithAuth(
  url: string,
  options: RequestInit = {},
): Promise<Response> {
  const token =
    typeof window !== "undefined" ? localStorage.getItem("access_token") : null;

  const headers: HeadersInit = {
    "Content-Type": "application/json",
    ...(token && { Authorization: `Bearer ${token}` }),
    ...options.headers,
  };

  const response = await fetch(url, {
    ...options,
    headers,
  });

  if (!response.ok) {
    const error = await response
      .json()
      .catch(() => ({ message: "An error occurred" }));
    throw new Error(error.message || `HTTP error! status: ${response.status}`);
  }

  return response;
}

// Consent Management

export async function getConsents(): Promise<ConsentResponse[]> {
  const response = await fetchWithAuth(`${API_BASE_URL}/api/gdpr/consents`);
  return response.json();
}

export async function updateConsent(
  request: UpdateConsentRequest,
): Promise<ConsentResponse> {
  const response = await fetchWithAuth(`${API_BASE_URL}/api/gdpr/consents`, {
    method: "PUT",
    body: JSON.stringify(request),
  });
  return response.json();
}

export async function batchUpdateConsents(
  consents: UpdateConsentRequest[],
): Promise<ConsentResponse[]> {
  const response = await fetchWithAuth(
    `${API_BASE_URL}/api/gdpr/consents/batch`,
    {
      method: "PUT",
      body: JSON.stringify({ consents }),
    },
  );
  return response.json();
}

// Data Export

export async function getExportRequests(): Promise<ExportRequestResponse[]> {
  const response = await fetchWithAuth(`${API_BASE_URL}/api/gdpr/export`);
  return response.json();
}

export async function createExportRequest(
  request: CreateExportRequest,
): Promise<ExportRequestResponse> {
  const response = await fetchWithAuth(`${API_BASE_URL}/api/gdpr/export`, {
    method: "POST",
    body: JSON.stringify(request),
  });
  return response.json();
}

export async function getExportRequest(
  id: string,
): Promise<ExportRequestResponse> {
  const response = await fetchWithAuth(`${API_BASE_URL}/api/gdpr/export/${id}`);
  return response.json();
}

export async function getExportableTypes(): Promise<ExportableDataTypes> {
  const response = await fetchWithAuth(`${API_BASE_URL}/api/gdpr/export-types`);
  return response.json();
}

// Data Deletion

export async function getDeletionRequests(): Promise<
  DeletionRequestResponse[]
> {
  const response = await fetchWithAuth(`${API_BASE_URL}/api/gdpr/deletion`);
  return response.json();
}

export async function createDeletionRequest(
  request: CreateDeletionRequest,
): Promise<DeletionRequestResponse> {
  const response = await fetchWithAuth(`${API_BASE_URL}/api/gdpr/deletion`, {
    method: "POST",
    body: JSON.stringify(request),
  });
  return response.json();
}

export async function getDeletionRequest(
  id: string,
): Promise<DeletionRequestResponse> {
  const response = await fetchWithAuth(
    `${API_BASE_URL}/api/gdpr/deletion/${id}`,
  );
  return response.json();
}

export async function cancelDeletionRequest(
  id: string,
): Promise<DeletionRequestResponse> {
  const response = await fetchWithAuth(
    `${API_BASE_URL}/api/gdpr/deletion/${id}/cancel`,
    {
      method: "POST",
    },
  );
  return response.json();
}

export async function confirmDeletion(
  confirmationToken: string,
): Promise<DeletionRequestResponse> {
  const response = await fetchWithAuth(
    `${API_BASE_URL}/api/gdpr/deletion/confirm`,
    {
      method: "POST",
      body: JSON.stringify({ confirmation_token: confirmationToken }),
    },
  );
  return response.json();
}

// GDPR Summary

export async function getGdprSummary(): Promise<GdprSummary> {
  const response = await fetchWithAuth(`${API_BASE_URL}/api/gdpr/summary`);
  return response.json();
}

// Consent type labels for display
export const CONSENT_LABELS: Record<
  string,
  { title: string; description: string }
> = {
  terms_of_service: {
    title: "Terms of Service",
    description: "I agree to the Terms of Service",
  },
  privacy_policy: {
    title: "Privacy Policy",
    description: "I have read and agree to the Privacy Policy",
  },
  marketing_emails: {
    title: "Marketing Emails",
    description: "Receive emails about product updates and promotions",
  },
  analytics: {
    title: "Analytics",
    description: "Allow usage analytics to improve the service",
  },
  personalization: {
    title: "Personalization",
    description: "Allow personalized recommendations and features",
  },
  data_sharing: {
    title: "Data Sharing",
    description: "Share data with partners for improved services",
  },
  cookies: {
    title: "Cookies",
    description: "Allow cookies for session management and analytics",
  },
};
