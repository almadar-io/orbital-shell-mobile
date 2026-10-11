import type { EventPayload, EventPayloadValue, JsonValue } from '@almadar/core';

const API_BASE_URL = process.env.EXPO_PUBLIC_API_URL || 'http://localhost:3000/api';

export interface OrbitalEventResponse {
  success: boolean;
  transitioned: boolean;
  states: Record<string, string>;
  emittedEvents: Array<{ event: string; payload?: EventPayload; source?: { orbital: string; trait: string } }>;
  clientEffects?: EventPayload[];
  clientEffectsByTrait?: Array<{ traitName: string; effect: EventPayload }>;
  effectResults?: Array<{
    effect: 'persist' | 'call-service' | 'set' | 'ref' | 'deref' | 'swap' | 'atomic';
    action?: string;
    entityType?: string;
    data?: EventPayloadValue;
    success: boolean;
    error?: string;
  }>;
  error?: string;
}

export interface ApiResponse<T> {
  data?: T;
  error?: string;
}

export const api = {
  async get<T>(path: string): Promise<ApiResponse<T>> {
    try {
      const response = await fetch(`${API_BASE_URL}${path}`);
      const data = await response.json();
      return { data };
    } catch (error) {
      return { error: error instanceof Error ? error.message : 'Unknown error' };
    }
  },

  async post<T>(path: string, body: JsonValue): Promise<ApiResponse<T>> {
    try {
      const response = await fetch(`${API_BASE_URL}${path}`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify(body),
      });
      const data = await response.json();
      return { data };
    } catch (error) {
      return { error: error instanceof Error ? error.message : 'Unknown error' };
    }
  },

  async sendEvent(
    orbital: string,
    event: string,
    payload?: EventPayload,
    entityId?: string,
    user?: { uid: string; email?: string; displayName?: string },
  ): Promise<ApiResponse<OrbitalEventResponse>> {
    return this.post<OrbitalEventResponse>(`/${orbital}/events`, {
      event,
      payload,
      entityId,
      user,
    });
  },
};
