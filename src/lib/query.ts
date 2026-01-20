import {
	useMutation,
	useQuery,
	useQueryClient,
	useSuspenseQuery,
} from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";
import i18n from "../i18n";

// Generate a readable workspace ID from title (e.g., "GSD" -> "ws_gsd", "My Config" -> "ws_my_config")
const generateWorkspaceId = (title: string): string => {
	const slug = title
		.toLowerCase()
		.trim()
		.replace(/[^a-z0-9]+/g, "_") // Replace non-alphanumeric with underscore
		.replace(/^_+|_+$/g, "") // Remove leading/trailing underscores
		.substring(0, 20); // Limit length
	return `ws_${slug || "workspace"}`;
};

export type ConfigType =
	| "user"
	| "enterprise_macos"
	| "enterprise_linux"
	| "enterprise_windows"
	| "mcp_macos"
	| "mcp_linux"
	| "mcp_windows";

export interface ConfigFile {
	path: string;
	content: unknown;
	exists: boolean;
}

export interface ClaudeSettings {
	model?: string;
	permissions?: Record<string, any>;
	env?: Record<string, string>;
	[key: string]: any;
}

export type WorkspaceType = "settings_only" | "full_directory";

export interface ConfigStore {
	id: string; // ws_<slug> format
	title: string;
	createdAt: number;
	settings: ClaudeSettings;
	using: boolean;
	// Workspace support
	workspaceType: WorkspaceType;
	workspacePath?: string;
	includeScripts: boolean;
	// Metadata for full directory workspaces
	skillsCount?: number;
	commandsCount?: number;
	agentsCount?: number;
	pluginsCount?: number;
	lastSynced?: number;
	// Git import source tracking
	sourceUrl?: string;
	// Git branch management - automatically checkout this branch when switching
	gitBranch?: string;
}

export interface GitImportPreview {
	repoName: string;
	hasSettingsJson: boolean;
	hasClaudeMd: boolean;
	hasMcpJson: boolean;
	skillsCount: number;
	commandsCount: number;
	agentsCount: number;
	pluginsCount: number;
	hasHooks: boolean;
	rootMdFiles: string[];
}

export interface ClaudeDirCounts {
	skills: number;
	commands: number;
	agents: number;
	plugins: number;
}

export type WorkspaceItemType = "skill" | "command" | "agent" | "hook" | "plugin";

export interface WorkspaceItem {
	name: string;
	relativePath: string;
	itemType: WorkspaceItemType;
}

export interface WorkspaceItems {
	skills: WorkspaceItem[];
	commands: WorkspaceItem[];
	agents: WorkspaceItem[];
	hooks: WorkspaceItem[];
	plugins: WorkspaceItem[];
}

export interface CopyItemsResult {
	copiedCount: number;
	failedItems: string[];
}

export interface WorkspaceSettings {
	content: Record<string, unknown>;
	referencedFiles: string[];
	exists: boolean;
}

export type SettingsMergeMode = "replace" | "merge";

export interface McpServer {
	config: Record<string, any>;
}

export interface NotificationSettings {
	enable: boolean;
	enabled_hooks: string[];
}

export interface CommandFile {
	name: string;
	content: string;
	exists: boolean;
}

export const useConfigFiles = () => {
	return useQuery({
		queryKey: ["config-files"],
		queryFn: () => invoke<ConfigType[]>("list_config_files"),
	});
};

export const useConfigFile = (configType: ConfigType) => {
	return useQuery({
		queryKey: ["config-file", configType],
		queryFn: () => invoke<ConfigFile>("read_config_file", { configType }),
		enabled: !!configType,
	});
};

export const useWriteConfigFile = () => {
	const queryClient = useQueryClient();

	return useMutation({
		mutationFn: ({
			configType,
			content,
		}: {
			configType: ConfigType;
			content: unknown;
		}) => invoke<void>("write_config_file", { configType, content }),
		onSuccess: (_, variables) => {
			toast.success(
				i18n.t("toast.configSaved", { configType: variables.configType }),
			);
			queryClient.invalidateQueries({
				queryKey: ["config-file", variables.configType],
			});
			queryClient.invalidateQueries({ queryKey: ["config-files"] });
		},
		onError: (error) => {
			const errorMessage =
				error instanceof Error ? error.message : String(error);
			toast.error(i18n.t("toast.configSaveFailed", { error: errorMessage }));
		},
	});
};

export const useBackupClaudeConfigs = () => {
	return useMutation({
		mutationFn: () => invoke<void>("backup_claude_configs"),
		onSuccess: () => {
			toast.success(i18n.t("toast.backupSuccess"));
		},
		onError: (error) => {
			const errorMessage =
				error instanceof Error ? error.message : String(error);
			toast.error(i18n.t("toast.backupFailed", { error: errorMessage }));
		},
	});
};

// Store management hooks

export const useStores = (options?: { storeId?: string }) => {
	return useSuspenseQuery({
		queryKey: ["stores", options?.storeId],
		queryFn: () => invoke<ConfigStore[]>("get_stores"),
	});
};

export const useStore = (storeId: string) => {
	return useSuspenseQuery({
		queryKey: ["store", storeId],
		queryFn: () => invoke<ConfigStore>("get_store", { storeId }),
	});
};

export const useCurrentStore = () => {
	return useSuspenseQuery({
		queryKey: ["current-store"],
		queryFn: () => invoke<ConfigStore | null>("get_current_store"),
	});
};

export const useClaudeDirCounts = () => {
	return useQuery({
		queryKey: ["claude-dir-counts"],
		queryFn: () => invoke<ClaudeDirCounts>("get_claude_dir_counts"),
		refetchInterval: 5000, // Refetch every 5 seconds to detect changes
	});
};

export const useCreateConfig = () => {
	const queryClient = useQueryClient();

	return useMutation({
		mutationFn: async ({
			title,
			settings,
			workspaceType,
			includeScripts,
		}: {
			title: string;
			settings: unknown;
			workspaceType?: WorkspaceType;
			includeScripts?: boolean;
		}) => {
			const id = generateWorkspaceId(title);
			return invoke<ConfigStore>("create_config", {
				id,
				title,
				settings,
				workspaceType: workspaceType ?? "settings_only",
				includeScripts: includeScripts ?? false,
			});
		},
		onSuccess: async () => {
			toast.success(i18n.t("toast.storeCreated"));
			queryClient.invalidateQueries({ queryKey: ["stores"] });
			queryClient.invalidateQueries({ queryKey: ["current-store"] });
			await rebuildTrayMenu();
		},
		onError: (error) => {
			const errorMessage =
				error instanceof Error ? error.message : String(error);
			toast.error(i18n.t("toast.storeCreateFailed", { error: errorMessage }));
		},
	});
};

export const useDeleteConfig = () => {
	const queryClient = useQueryClient();

	return useMutation({
		mutationFn: (body: { storeId: string }) =>
			invoke<void>("delete_config", {
				storeId: body.storeId,
			}),
		onSuccess: async () => {
			toast.success(i18n.t("toast.storeDeleted"));
			queryClient.invalidateQueries({ queryKey: ["stores"] });
			queryClient.invalidateQueries({ queryKey: ["current-store"] });
			await rebuildTrayMenu();
		},
		onError: (error) => {
			const errorMessage =
				error instanceof Error ? error.message : String(error);
			toast.error(i18n.t("toast.storeDeleteFailed", { error: errorMessage }));
		},
	});
};

export const useSetUsingConfig = () => {
	const queryClient = useQueryClient();

	return useMutation({
		mutationFn: (storeId: string) =>
			invoke<void>("set_using_config", { storeId }),
		onSuccess: () => {
			toast.success(i18n.t("toast.storeActivated"));
			queryClient.invalidateQueries({ queryKey: ["stores"] });
			queryClient.invalidateQueries({ queryKey: ["current-store"] });
			queryClient.invalidateQueries({ queryKey: ["config-file", "user"] });
		},
		onError: (error) => {
			const errorMessage =
				error instanceof Error ? error.message : String(error);
			toast.error(i18n.t("toast.storeActivateFailed", { error: errorMessage }));
		},
	});
};

export const useSetCurrentConfig = () => {
	const queryClient = useQueryClient();

	return useMutation({
		mutationFn: (storeId: string) => {
			console.log("🔴🔴🔴 SET_USING_CONFIG CALLED! storeId:", storeId);
			console.log("🔴🔴🔴 TIMESTAMP:", new Date().toISOString());
			console.trace("Stack trace for set_using_config:");
			return invoke<void>("set_using_config", { storeId });
		},
		onSuccess: () => {
			console.log("🔴🔴🔴 SET_USING_CONFIG SUCCESS - this clears ~/.claude!");
			queryClient.invalidateQueries({ queryKey: ["stores"] });
			queryClient.invalidateQueries({ queryKey: ["current-store"] });
			queryClient.invalidateQueries({ queryKey: ["config-file", "user"] });
		},
	});
};

export const useResetToOriginalConfig = () => {
	const queryClient = useQueryClient();

	return useMutation({
		mutationFn: () => invoke<void>("reset_to_original_config"),
		onSuccess: () => {
			queryClient.invalidateQueries({ queryKey: ["stores"] });
			queryClient.invalidateQueries({ queryKey: ["current-store"] });
			queryClient.invalidateQueries({ queryKey: ["config-file", "user"] });
		},
	});
};

export const useUpdateConfig = () => {
	const queryClient = useQueryClient();

	return useMutation({
		mutationFn: ({
			storeId,
			title,
			settings,
			gitBranch,
		}: {
			storeId: string;
			title: string;
			settings: unknown;
			gitBranch?: string | null;
		}) => invoke<ConfigStore>("update_config", { storeId, title, settings, gitBranch }),
		onSuccess: async (data) => {
			if (data.using) {
				toast.success(
					i18n.t("toast.storeSavedAndActive", { title: data.title }),
				);
			} else {
				toast.success(i18n.t("toast.storeSaved", { title: data.title }));
			}
			queryClient.invalidateQueries({ queryKey: ["stores"] });
			queryClient.invalidateQueries({ queryKey: ["store", data.id] });
			queryClient.invalidateQueries({ queryKey: ["current-store"] });
			if (data.using) {
				queryClient.invalidateQueries({ queryKey: ["config-file", "user"] });
			}
			await rebuildTrayMenu();
		},
		onError: (error) => {
			const errorMessage =
				error instanceof Error ? error.message : String(error);
			toast.error(i18n.t("toast.storeSaveFailed", { error: errorMessage }));
		},
	});
};

export interface UpdateInfo {
	available: boolean;
	version?: string;
	body?: string;
	date?: string;
}

export const useCheckForUpdates = () => {
	return useQuery({
		queryKey: ["check-updates"],
		queryFn: () => invoke<UpdateInfo>("check_for_updates"),
		refetchInterval: 1000 * 60 * 30, // Check every 30 minutes
		retry: 1,
		refetchOnWindowFocus: false,
	});
};

export const useInstallAndRestart = () => {
	return useMutation({
		mutationFn: () => {
			console.log("🚀 Frontend: Starting update installation");
			return invoke<void>("install_and_restart");
		},
		onSuccess: () => {
			console.log(
				"✅ Frontend: Update installation completed, preparing to restart",
			);
			toast.success(i18n.t("toast.updateInstalled"));
		},
		onError: (error) => {
			console.log("❌ Frontend: Update installation failed", error);
			const errorMessage =
				error instanceof Error ? error.message : String(error);
			toast.error(i18n.t("toast.updateInstallFailed", { error: errorMessage }));
		},
	});
};

// MCP Server management hooks

export const useGlobalMcpServers = () => {
	return useSuspenseQuery({
		queryKey: ["global-mcp-servers"],
		queryFn: () => invoke<Record<string, McpServer>>("get_global_mcp_servers"),
	});
};

export const useUpdateGlobalMcpServer = () => {
	const queryClient = useQueryClient();

	return useMutation({
		mutationFn: ({
			serverName,
			serverConfig,
		}: {
			serverName: string;
			serverConfig: Record<string, any>;
		}) =>
			invoke<void>("update_global_mcp_server", { serverName, serverConfig }),
		onSuccess: () => {
			toast.success("MCP server configuration updated successfully");
			queryClient.invalidateQueries({ queryKey: ["global-mcp-servers"] });
		},
		onError: (error) => {
			const errorMessage =
				error instanceof Error ? error.message : String(error);
			toast.error(`Failed to update MCP server: ${errorMessage}`);
		},
	});
};

export const useAddGlobalMcpServer = () => {
	const queryClient = useQueryClient();

	return useMutation({
		mutationFn: ({
			serverName,
			serverConfig,
		}: {
			serverName: string;
			serverConfig: Record<string, any>;
		}) =>
			invoke<void>("update_global_mcp_server", { serverName, serverConfig }),
		onSuccess: () => {
			toast.success("MCP server added successfully");
			queryClient.invalidateQueries({ queryKey: ["global-mcp-servers"] });
		},
		onError: (error) => {
			const errorMessage =
				error instanceof Error ? error.message : String(error);
			toast.error(`Failed to add MCP server: ${errorMessage}`);
		},
	});
};

export const useCheckMcpServerExists = (
	serverName: string,
	options?: { enabled?: boolean },
) => {
	return useQuery({
		queryKey: ["check-mcp-server-exists", serverName],
		queryFn: () => invoke<boolean>("check_mcp_server_exists", { serverName }),
		enabled: options?.enabled !== false && !!serverName, // Only run when serverName is provided
	});
};

export const useDeleteGlobalMcpServer = () => {
	const queryClient = useQueryClient();

	return useMutation({
		mutationFn: (serverName: string) =>
			invoke<void>("delete_global_mcp_server", { serverName }),
		onSuccess: () => {
			toast.success("MCP server deleted successfully");
			queryClient.invalidateQueries({ queryKey: ["global-mcp-servers"] });
		},
		onError: (error) => {
			const errorMessage =
				error instanceof Error ? error.message : String(error);
			toast.error(`Failed to delete MCP server: ${errorMessage}`);
		},
	});
};

export interface UsageData {
	input_tokens?: number;
	cache_read_input_tokens?: number;
	output_tokens?: number;
}

export interface ProjectUsageRecord {
	uuid: string;
	timestamp: string;
	model?: string;
	usage?: UsageData;
}

export const useProjectUsageFiles = () => {
	return useQuery({
		queryKey: ["project-usage-files"],
		queryFn: () => invoke<ProjectUsageRecord[]>("read_project_usage_files"),
	});
};

// Memory management hooks

export interface MemoryFile {
	path: string;
	content: string;
	exists: boolean;
}

export const useClaudeMemory = () => {
	return useSuspenseQuery({
		queryKey: ["claude-memory"],
		queryFn: () => invoke<MemoryFile>("read_claude_memory"),
	});
};

export const useWriteClaudeMemory = () => {
	const queryClient = useQueryClient();

	return useMutation({
		mutationFn: (content: string) =>
			invoke<void>("write_claude_memory", { content }),
		onSuccess: () => {
			toast.success("Memory saved successfully");
			queryClient.invalidateQueries({ queryKey: ["claude-memory"] });
		},
		onError: (error) => {
			const errorMessage =
				error instanceof Error ? error.message : String(error);
			toast.error(`Failed to save memory: ${errorMessage}`);
		},
	});
};

// Projects management hooks

export interface ProjectConfig {
	path: string;
	config: Record<string, any>;
}

export const useClaudeProjects = () => {
	return useQuery({
		queryKey: ["claude-projects"],
		queryFn: () => invoke<ProjectConfig[]>("read_claude_projects"),
	});
};

export interface ClaudeConfigFile {
	path: string;
	content: unknown;
	exists: boolean;
}

export const useClaudeConfigFile = () => {
	return useQuery({
		queryKey: ["claude-config-file"],
		queryFn: () => invoke<ClaudeConfigFile>("read_claude_config_file"),
	});
};

export const useWriteClaudeConfigFile = () => {
	const queryClient = useQueryClient();

	return useMutation({
		mutationFn: (content: unknown) =>
			invoke<void>("write_claude_config_file", { content }),
		onSuccess: () => {
			toast.success("Claude configuration saved successfully");
			queryClient.invalidateQueries({ queryKey: ["claude-config-file"] });
			queryClient.invalidateQueries({ queryKey: ["claude-projects"] });
		},
		onError: (error) => {
			const errorMessage =
				error instanceof Error ? error.message : String(error);
			toast.error(`Failed to save Claude configuration: ${errorMessage}`);
		},
	});
};

// Notification settings hooks

export const useNotificationSettings = () => {
	return useQuery({
		queryKey: ["notification-settings"],
		queryFn: () =>
			invoke<NotificationSettings | null>("get_notification_settings"),
	});
};

export const useUpdateNotificationSettings = () => {
	const queryClient = useQueryClient();

	return useMutation({
		mutationFn: (settings: NotificationSettings) => {
			return invoke<void>("update_notification_settings", { settings });
		},
		onSuccess: () => {
			toast.success("Notification settings updated successfully");
			queryClient.invalidateQueries({ queryKey: ["notification-settings"] });
		},
		onError: (error) => {
			const errorMessage =
				error instanceof Error ? error.message : String(error);
			toast.error(`Failed to update notification settings: ${errorMessage}`);
		},
	});
};

// Command management hooks
export const useClaudeCommands = () =>
	useQuery({
		queryKey: ["claude-commands"],
		queryFn: () => invoke<CommandFile[]>("read_claude_commands"),
	});

export const useWriteClaudeCommand = () => {
	const queryClient = useQueryClient();

	return useMutation({
		mutationFn: ({
			commandName,
			content,
		}: {
			commandName: string;
			content: string;
		}) => invoke<void>("write_claude_command", { commandName, content }),
		onSuccess: () => {
			toast.success(i18n.t("toast.commandSaved"));
			queryClient.invalidateQueries({ queryKey: ["claude-commands"] });
		},
		onError: (error) => {
			const errorMessage =
				error instanceof Error ? error.message : String(error);
			toast.error(i18n.t("toast.commandSaveFailed", { error: errorMessage }));
		},
	});
};

export const useDeleteClaudeCommand = () => {
	const queryClient = useQueryClient();

	return useMutation({
		mutationFn: (commandName: string) =>
			invoke<void>("delete_claude_command", { commandName }),
		onSuccess: () => {
			toast.success(i18n.t("toast.commandDeleted"));
			queryClient.invalidateQueries({ queryKey: ["claude-commands"] });
		},
		onError: (error) => {
			const errorMessage =
				error instanceof Error ? error.message : String(error);
			toast.error(i18n.t("toast.commandDeleteFailed", { error: errorMessage }));
		},
	});
};

// Agent management hooks
export const useClaudeAgents = () =>
	useQuery({
		queryKey: ["claude-agents"],
		queryFn: () => invoke<CommandFile[]>("read_claude_agents"),
	});

export const useWriteClaudeAgent = () => {
	const queryClient = useQueryClient();

	return useMutation({
		mutationFn: ({
			agentName,
			content,
		}: {
			agentName: string;
			content: string;
		}) => invoke<void>("write_claude_agent", { agentName, content }),
		onSuccess: () => {
			toast.success("Agent saved successfully");
			queryClient.invalidateQueries({ queryKey: ["claude-agents"] });
		},
		onError: (error) => {
			const errorMessage =
				error instanceof Error ? error.message : String(error);
			toast.error(`Failed to save agent: ${errorMessage}`);
		},
	});
};

export const useDeleteClaudeAgent = () => {
	const queryClient = useQueryClient();

	return useMutation({
		mutationFn: (agentName: string) =>
			invoke<void>("delete_claude_agent", { agentName }),
		onSuccess: () => {
			toast.success("Agent deleted successfully");
			queryClient.invalidateQueries({ queryKey: ["claude-agents"] });
		},
		onError: (error) => {
			const errorMessage =
				error instanceof Error ? error.message : String(error);
			toast.error(`Failed to delete agent: ${errorMessage}`);
		},
	});
};

// Skill management hooks
export interface SkillFile {
	name: string;
	content: string;
	references_count: number;
}

export const useClaudeSkills = () =>
	useQuery({
		queryKey: ["claude-skills"],
		queryFn: () => invoke<SkillFile[]>("read_claude_skills"),
	});

export const useWriteClaudeSkill = () => {
	const queryClient = useQueryClient();

	return useMutation({
		mutationFn: ({
			skillName,
			content,
		}: {
			skillName: string;
			content: string;
		}) => invoke<void>("write_claude_skill", { skillName, content }),
		onSuccess: () => {
			toast.success("Skill saved successfully");
			queryClient.invalidateQueries({ queryKey: ["claude-skills"] });
		},
		onError: (error) => {
			const errorMessage =
				error instanceof Error ? error.message : String(error);
			toast.error(`Failed to save skill: ${errorMessage}`);
		},
	});
};

export const useDeleteClaudeSkill = () => {
	const queryClient = useQueryClient();

	return useMutation({
		mutationFn: (skillName: string) =>
			invoke<void>("delete_claude_skill", { skillName }),
		onSuccess: () => {
			toast.success("Skill deleted successfully");
			queryClient.invalidateQueries({ queryKey: ["claude-skills"] });
		},
		onError: (error) => {
			const errorMessage =
				error instanceof Error ? error.message : String(error);
			toast.error(`Failed to delete skill: ${errorMessage}`);
		},
	});
};

// Plugin management hooks
export interface PluginInfo {
	name: string;
	path: string;
	is_local: boolean;
	is_enabled: boolean;
	has_mcp: boolean;
	commands_count: number;
	agents_count: number;
	skills_count: number;
	description: string | null;
}

export const useClaudePlugins = () =>
	useQuery({
		queryKey: ["claude-plugins"],
		queryFn: () => invoke<PluginInfo[]>("read_claude_plugins"),
	});

export const useTogglePlugin = () => {
	const queryClient = useQueryClient();

	return useMutation({
		mutationFn: ({
			pluginPath,
			enabled,
		}: {
			pluginPath: string;
			enabled: boolean;
		}) => invoke<void>("toggle_plugin", { pluginPath, enabled }),
		onSuccess: () => {
			queryClient.invalidateQueries({ queryKey: ["claude-plugins"] });
		},
		onError: (error) => {
			const errorMessage =
				error instanceof Error ? error.message : String(error);
			toast.error(i18n.t("toast.pluginToggleFailed", { error: errorMessage }));
		},
	});
};

export const useDeleteLocalPlugin = () => {
	const queryClient = useQueryClient();

	return useMutation({
		mutationFn: (pluginName: string) =>
			invoke<void>("delete_local_plugin", { pluginName }),
		onSuccess: () => {
			toast.success(i18n.t("toast.pluginDeleted"));
			queryClient.invalidateQueries({ queryKey: ["claude-plugins"] });
		},
		onError: (error) => {
			const errorMessage =
				error instanceof Error ? error.message : String(error);
			toast.error(i18n.t("toast.pluginDeleteFailed", { error: errorMessage }));
		},
	});
};

// Sync workspace from current ~/.claude state
export const useSyncWorkspaceFromClaude = () => {
	const queryClient = useQueryClient();

	return useMutation({
		mutationFn: (storeId: string) =>
			invoke<void>("sync_workspace_from_claude", { storeId }),
		onSuccess: () => {
			toast.success(i18n.t("toast.workspaceSynced"));
			queryClient.invalidateQueries({ queryKey: ["stores"] });
			queryClient.invalidateQueries({ queryKey: ["current-store"] });
		},
		onError: (error) => {
			const errorMessage =
				error instanceof Error ? error.message : String(error);
			toast.error(i18n.t("toast.workspaceSyncFailed", { error: errorMessage }));
		},
	});
};

// Git import hooks

export const usePreviewGitImport = () => {
	return useMutation({
		mutationFn: (url: string) =>
			invoke<GitImportPreview>("preview_git_import", { url }),
	});
};

export const useImportWorkspaceFromGit = () => {
	const queryClient = useQueryClient();

	return useMutation({
		mutationFn: async ({ url, title }: { url: string; title: string }) => {
			const id = generateWorkspaceId(title);
			return invoke<ConfigStore>("import_workspace_from_git", {
				url,
				title,
				id,
			});
		},
		onSuccess: async () => {
			toast.success(i18n.t("toast.workspaceImported"));
			queryClient.invalidateQueries({ queryKey: ["stores"] });
			queryClient.invalidateQueries({ queryKey: ["current-store"] });
			await rebuildTrayMenu();
		},
		onError: (error) => {
			const errorMessage =
				error instanceof Error ? error.message : String(error);
			toast.error(i18n.t("toast.workspaceImportFailed", { error: errorMessage }));
		},
	});
};

// Cherry-pick hooks

export const useWorkspaceItems = (
	workspaceId: string,
	options?: { enabled?: boolean },
) => {
	return useQuery({
		queryKey: ["workspace-items", workspaceId],
		queryFn: () =>
			invoke<WorkspaceItems>("list_workspace_items", { workspaceId }),
		enabled: options?.enabled !== false && !!workspaceId,
	});
};

export const useCopyItemsToClaude = () => {
	const queryClient = useQueryClient();

	return useMutation({
		mutationFn: ({
			sourceWorkspaceId,
			items,
		}: {
			sourceWorkspaceId: string;
			items: WorkspaceItem[];
		}) =>
			invoke<CopyItemsResult>("copy_items_to_claude", {
				sourceWorkspaceId,
				items,
			}),
		onSuccess: (result) => {
			if (result.copiedCount > 0) {
				toast.success(
					i18n.t("toast.itemsCopied", { count: result.copiedCount }),
				);
			}
			if (result.failedItems.length > 0) {
				toast.error(
					i18n.t("toast.itemsCopyFailed", {
						count: result.failedItems.length,
					}),
				);
			}
			// Only invalidate claude-dir-counts to refresh live counts
			// Do NOT invalidate stores/current-store to avoid any workspace switching
			queryClient.invalidateQueries({ queryKey: ["claude-dir-counts"] });
		},
		onError: (error) => {
			const errorMessage =
				error instanceof Error ? error.message : String(error);
			toast.error(i18n.t("toast.itemsCopyError", { error: errorMessage }));
		},
	});
};

export const useWorkspaceSettings = (
	workspaceId: string,
	options?: { enabled?: boolean },
) => {
	return useQuery({
		queryKey: ["workspace-settings", workspaceId],
		queryFn: () =>
			invoke<WorkspaceSettings>("get_workspace_settings", { workspaceId }),
		enabled: options?.enabled !== false && !!workspaceId,
	});
};

export const useCopyWorkspaceSettings = () => {
	const queryClient = useQueryClient();

	return useMutation({
		mutationFn: ({
			sourceWorkspaceId,
			mode,
			copyReferencedFiles,
		}: {
			sourceWorkspaceId: string;
			mode: SettingsMergeMode;
			copyReferencedFiles: boolean;
		}) => {
			console.log("🔵 CHERRY-PICK SETTINGS: Starting mutation", { sourceWorkspaceId, mode, copyReferencedFiles });
			return invoke<CopyItemsResult>("copy_workspace_settings", {
				sourceWorkspaceId,
				mode,
				copyReferencedFiles,
			});
		},
		onSuccess: (result) => {
			console.log("🟢 CHERRY-PICK SETTINGS: Success!", result);
			if (result.copiedCount > 0) {
				toast.success(
					i18n.t("toast.settingsCopied", { count: result.copiedCount }),
				);
			}
			if (result.failedItems.length > 0) {
				toast.error(
					i18n.t("toast.settingsCopyFailed", {
						count: result.failedItems.length,
					}),
				);
			}
			// Only invalidate config-file to show updated settings
			// Do NOT invalidate stores/current-store to avoid any workspace switching
			console.log("🟢 CHERRY-PICK SETTINGS: Invalidating ONLY config-file");
			queryClient.invalidateQueries({ queryKey: ["config-file", "user"] });
			console.log("🟢 CHERRY-PICK SETTINGS: Done - NO stores/current-store invalidation");
		},
		onError: (error) => {
			console.log("🔴 CHERRY-PICK SETTINGS: Error!", error);
			const errorMessage =
				error instanceof Error ? error.message : String(error);
			toast.error(i18n.t("toast.settingsCopyError", { error: errorMessage }));
		},
	});
};

// Helper function to rebuild tray menu
const rebuildTrayMenu = async () => {
	try {
		await invoke<void>("rebuild_tray_menu_command");
	} catch (error) {
		console.error("Failed to rebuild tray menu:", error);
	}
};

// ============================================================================
// Git Branch Hooks
// ============================================================================

export const useGitBranches = () => {
	return useQuery({
		queryKey: ["git-branches"],
		queryFn: () => invoke<string[]>("list_git_branches"),
	});
};

export const useCurrentGitBranch = () => {
	return useQuery({
		queryKey: ["current-git-branch"],
		queryFn: () => invoke<string>("get_current_git_branch"),
	});
};

export const useCreateGitBranch = () => {
	const queryClient = useQueryClient();

	return useMutation({
		mutationFn: (branchName: string) =>
			invoke<void>("create_git_branch", { branchName }),
		onSuccess: () => {
			queryClient.invalidateQueries({ queryKey: ["git-branches"] });
		},
	});
};
