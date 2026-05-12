import { browser } from '$app/environment';

export type DesktopBackendStatusKind = 'starting' | 'running' | 'failed' | 'stopped';

export type DesktopBackendStatus = {
	kind: DesktopBackendStatusKind;
	resp_port: number;
	http_port: number;
	message: string;
};

export function isTauriRuntime(): boolean {
	return browser && '__TAURI_INTERNALS__' in window;
}

export async function readDesktopBackendStatus(): Promise<DesktopBackendStatus | null> {
	if (!isTauriRuntime()) {
		return null;
	}
	try {
		const { invoke } = await import('@tauri-apps/api/core');
		return await invoke<DesktopBackendStatus>('desktop_backend_status');
	} catch (error) {
		return {
			kind: 'failed',
			resp_port: 6380,
			http_port: 6381,
			message: `Failed to read Tauri sidecar status: ${String(error)}`
		};
	}
}

export function apiPath(path: string): string {
	if (!isTauriRuntime()) {
		return path;
	}
	return `http://127.0.0.1:6381${path}`;
}
