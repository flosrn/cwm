import { useTranslation } from "react-i18next";
import { Label } from "@/components/ui/label";
import { Skeleton } from "@/components/ui/skeleton";
import { Switch } from "@/components/ui/switch";
import {
	useNotificationSettings,
	useUpdateNotificationSettings,
} from "@/lib/query";

export function NotificationPage() {
	const { t } = useTranslation();
	const { data: notificationSettings, isLoading } = useNotificationSettings();
	const updateNotificationSettings = useUpdateNotificationSettings();

	const handleGeneralToggle = (checked: boolean) => {
		if (!notificationSettings) return;

		const newSettings = {
			enable: checked,
			enabled_hooks: checked ? notificationSettings.enabled_hooks : [],
		};
		updateNotificationSettings.mutate(newSettings);
	};

	const handleHookToggle = (hookName: string, checked: boolean) => {
		if (!notificationSettings) return;

		let newHooks: string[];
		if (checked) {
			newHooks = [...notificationSettings.enabled_hooks, hookName];
		} else {
			newHooks = notificationSettings.enabled_hooks.filter(
				(hook) => hook !== hookName,
			);
		}

		const newSettings = {
			enable: notificationSettings.enable,
			enabled_hooks: newHooks,
		};
		updateNotificationSettings.mutate(newSettings);
	};

	const isHookEnabled = (hookName: string) => {
		return notificationSettings?.enabled_hooks.includes(hookName) || false;
	};

	if (isLoading) {
		return (
			<div className="">
				<div
					className="flex items-center p-3 border-b px-3 justify-between sticky top-0 bg-background z-10 mb-4"
					data-tauri-drag-region
				>
					<div data-tauri-drag-region>
						<Skeleton className="h-6 w-32 mb-1" />
						<Skeleton className="h-4 w-48" />
					</div>
				</div>
				<div className="px-4 flex flex-col bg-card mx-4 rounded-lg py-1 border space-y-4">
					<div className="border-b px-1 py-3">
						<div className="flex items-center justify-between">
							<Skeleton className="h-4 w-24" />
							<Skeleton className="h-6 w-11" />
						</div>
						<Skeleton className="h-3 w-64 mt-2" />
					</div>
					<div className="border-b px-1 py-3">
						<div className="flex items-center justify-between">
							<Skeleton className="h-4 w-24" />
							<Skeleton className="h-6 w-11" />
						</div>
						<Skeleton className="h-3 w-64 mt-2" />
					</div>
					<div className="px-1 py-3">
						<div className="flex items-center justify-between">
							<Skeleton className="h-4 w-24" />
							<Skeleton className="h-6 w-11" />
						</div>
						<Skeleton className="h-3 w-64 mt-2" />
					</div>
				</div>
			</div>
		);
	}

	return (
		<div className="">
			<div
				className="flex items-center p-3 border-b px-3 justify-between sticky top-0 bg-background z-10 mb-4"
				data-tauri-drag-region
			>
				<div data-tauri-drag-region>
					<h3 className="font-bold" data-tauri-drag-region>
						{t("notifications.title")}
					</h3>
					<p className="text-sm text-muted-foreground" data-tauri-drag-region>
						{t("notifications.description")}
					</p>
				</div>
			</div>
			<div className="px-4 flex flex-col bg-card mx-4 rounded-lg py-1 border">
				<div className="border-b px-1 py-3">
					<div className="flex items-center justify-between">
						<Label htmlFor="notification" className="">
							{t("notifications.general")}
						</Label>
						<Switch
							id="notification"
							checked={notificationSettings?.enable || false}
							onCheckedChange={handleGeneralToggle}
						/>
					</div>
					<div className="text-muted-foreground text-sm">
						{t("notifications.generalDescription")}
					</div>
				</div>
				<div className="border-b px-1 py-3">
					<div className="flex items-center justify-between">
						<Label htmlFor="preToolUse" className="">
							{t("notifications.toolUse")}
						</Label>
						<Switch
							id="preToolUse"
							checked={isHookEnabled("PreToolUse")}
							onCheckedChange={(checked) =>
								handleHookToggle("PreToolUse", checked)
							}
							disabled={!notificationSettings?.enable}
						/>
					</div>
					<div className="text-muted-foreground text-sm">
						{t("notifications.toolUseDescription")}
					</div>
				</div>
				<div className="px-1 py-3">
					<div className="flex items-center justify-between">
						<Label htmlFor="stop" className="">
							{t("notifications.completion")}
						</Label>
						<Switch
							id="stop"
							checked={isHookEnabled("Stop")}
							onCheckedChange={(checked) => handleHookToggle("Stop", checked)}
							disabled={!notificationSettings?.enable}
						/>
					</div>
					<div className="text-muted-foreground text-sm">
						{t("notifications.completionDescription")}
					</div>
				</div>
			</div>
		</div>
	);
}
