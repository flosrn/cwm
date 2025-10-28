import { invoke } from "@tauri-apps/api/core";

export enum TrackEvent {
	AppLaunched = "app_launched",
}

// Analytics tracking function
export const track = async (
	event: string,
	properties: Record<string, any> = {},
) => {
	if (!import.meta.env.PROD) {
		return;
	}
	try {
		await invoke<void>("track", { event, properties });
	} catch (error) {
		console.error("Failed to track event:", error);
	}
};
