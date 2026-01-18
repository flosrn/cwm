import { Kimi, Minimax, ZAI } from "@lobehub/icons";
import {
	AlertTriangleIcon,
	EllipsisVerticalIcon,
	FolderIcon,
	InfoIcon,
	PencilLineIcon,
	PlusIcon,
	RefreshCwIcon,
	SettingsIcon,
} from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router-dom";
import { GLMDialog } from "@/components/GLMBanner";
import { KimiDialog } from "@/components/KimiDialog";
import { MiniMaxDialog } from "@/components/MiniMaxDialog";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { ButtonGroup } from "@/components/ui/button-group";
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
	DialogTrigger,
} from "@/components/ui/dialog";
import {
	DropdownMenu,
	DropdownMenuContent,
	DropdownMenuItem,
	DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import {
	Tooltip,
	TooltipContent,
	TooltipProvider,
	TooltipTrigger,
} from "@/components/ui/tooltip";
import { cn } from "@/lib/utils";
import {
	type WorkspaceType,
	useCreateConfig,
	useResetToOriginalConfig,
	useSetCurrentConfig,
	useStores,
	useSyncWorkspaceFromClaude,
} from "../lib/query";

export function ConfigSwitcherPage() {
	return (
		<div className="">
			<section>
				<ConfigStores />
			</section>
		</div>
	);
}

function ConfigStores() {
	const { t } = useTranslation();
	const { data: stores } = useStores();
	const setCurrentStoreMutation = useSetCurrentConfig();
	const resetToOriginalMutation = useResetToOriginalConfig();
	const syncWorkspaceMutation = useSyncWorkspaceFromClaude();
	const navigate = useNavigate();

	// New workspace dialog state
	const [isCreateDialogOpen, setIsCreateDialogOpen] = useState(false);
	const [newWorkspaceTitle, setNewWorkspaceTitle] = useState("");
	const [isFullDirectory, setIsFullDirectory] = useState(true);
	const [includeScripts, setIncludeScripts] = useState(false);

	// Confirmation dialog state for workspace switching
	const [isSwitchDialogOpen, setIsSwitchDialogOpen] = useState(false);
	const [pendingSwitchStoreId, setPendingSwitchStoreId] = useState<string | null>(null);
	const [pendingSwitchToOriginal, setPendingSwitchToOriginal] = useState(false);

	const isOriginalConfigActive = !stores.some((store) => store.using);

	const handleStoreClick = (storeId: string, isCurrentStore: boolean) => {
		if (!isCurrentStore) {
			// Show confirmation dialog before switching
			setPendingSwitchStoreId(storeId);
			setPendingSwitchToOriginal(false);
			setIsSwitchDialogOpen(true);
		}
	};

	const handleOriginalConfigClick = () => {
		if (!isOriginalConfigActive) {
			// Show confirmation dialog before switching
			setPendingSwitchStoreId(null);
			setPendingSwitchToOriginal(true);
			setIsSwitchDialogOpen(true);
		}
	};

	const confirmSwitch = () => {
		if (pendingSwitchToOriginal) {
			resetToOriginalMutation.mutate();
		} else if (pendingSwitchStoreId) {
			setCurrentStoreMutation.mutate(pendingSwitchStoreId);
		}
		setIsSwitchDialogOpen(false);
		setPendingSwitchStoreId(null);
		setPendingSwitchToOriginal(false);
	};

	const cancelSwitch = () => {
		setIsSwitchDialogOpen(false);
		setPendingSwitchStoreId(null);
		setPendingSwitchToOriginal(false);
	};

	const handleSyncWorkspace = (e: React.MouseEvent, storeId: string) => {
		e.stopPropagation();
		syncWorkspaceMutation.mutate(storeId);
	};

	const createStoreMutation = useCreateConfig();

	const onCreateStore = async () => {
		const workspaceType: WorkspaceType = isFullDirectory
			? "full_directory"
			: "settings_only";
		const store = await createStoreMutation.mutateAsync({
			title: newWorkspaceTitle || t("configSwitcher.newConfig"),
			settings: {},
			workspaceType,
			includeScripts,
		});
		setIsCreateDialogOpen(false);
		setNewWorkspaceTitle("");
		setIsFullDirectory(true);
		setIncludeScripts(false);
		navigate(`/edit/${store.id}`);
	};

	const openCreateDialog = () => {
		setNewWorkspaceTitle("");
		setIsFullDirectory(true);
		setIncludeScripts(false);
		setIsCreateDialogOpen(true);
	};

	if (stores.length === 0) {
		return (
			<TooltipProvider>
				<div
					className="flex justify-center items-center h-screen"
					data-tauri-drag-region
				>
					<div className="flex flex-col items-center gap-2">
						<Dialog
							open={isCreateDialogOpen}
							onOpenChange={setIsCreateDialogOpen}
						>
							<DialogTrigger asChild>
								<Button variant="ghost" onClick={openCreateDialog}>
									<PlusIcon size={14} />
									{t("configSwitcher.createConfig")}
								</Button>
							</DialogTrigger>
							<DialogContent>
								<DialogHeader>
									<DialogTitle>{t("workspace.createWorkspace")}</DialogTitle>
									<DialogDescription>
										{t("workspace.createWorkspaceDescription")}
									</DialogDescription>
								</DialogHeader>
								<div className="space-y-4 py-4">
									<div className="space-y-2">
										<Label htmlFor="title">
											{t("workspace.workspaceName")}
										</Label>
										<Input
											id="title"
											placeholder={t("configSwitcher.newConfig")}
											value={newWorkspaceTitle}
											onChange={(e) => setNewWorkspaceTitle(e.target.value)}
										/>
									</div>

									<div className="flex items-center justify-between">
										<div className="flex items-center gap-2">
											<Label htmlFor="fullDirectory">
												{t("workspace.fullDirectory")}
											</Label>
											<Tooltip>
												<TooltipTrigger>
													<InfoIcon className="h-4 w-4 text-muted-foreground" />
												</TooltipTrigger>
												<TooltipContent className="max-w-xs">
													{t("workspace.fullDirectoryTooltip")}
												</TooltipContent>
											</Tooltip>
										</div>
										<Switch
											id="fullDirectory"
											checked={isFullDirectory}
											onCheckedChange={setIsFullDirectory}
										/>
									</div>

									{isFullDirectory && (
										<div className="flex items-center justify-between pl-4">
											<div className="flex items-center gap-2">
												<Label htmlFor="includeScripts">
													{t("workspace.includeScripts")}
												</Label>
												<Tooltip>
													<TooltipTrigger>
														<InfoIcon className="h-4 w-4 text-muted-foreground" />
													</TooltipTrigger>
													<TooltipContent className="max-w-xs">
														{t("workspace.includeScriptsTooltip")}
													</TooltipContent>
												</Tooltip>
											</div>
											<Switch
												id="includeScripts"
												checked={includeScripts}
												onCheckedChange={setIncludeScripts}
											/>
										</div>
									)}
								</div>
								<DialogFooter>
									<Button
										onClick={onCreateStore}
										disabled={createStoreMutation.isPending}
									>
										{createStoreMutation.isPending
											? t("workspace.creating")
											: t("workspace.create")}
									</Button>
								</DialogFooter>
							</DialogContent>
						</Dialog>

						<p className="text-sm text-muted-foreground" data-tauri-drag-region>
							{t("configSwitcher.description")}
						</p>

						<div className="mt-4 space-y-2">
							<GLMDialog
								trigger={
									<Button
										variant="ghost"
										className="text-muted-foreground text-sm"
										size="sm"
									>
										<ZAI />
										{t("glm.useZhipuGlm")}
									</Button>
								}
							/>
							<MiniMaxDialog
								trigger={
									<Button
										variant="ghost"
										className="text-muted-foreground text-sm"
										size="sm"
									>
										<Minimax />
										{t("minimax.useMiniMax")}
									</Button>
								}
							/>
							<KimiDialog
								trigger={
									<Button
										variant="ghost"
										className="text-muted-foreground text-sm"
										size="sm"
									>
										<Kimi />
										{t("kimi.useKimi")}
									</Button>
								}
							/>
						</div>
					</div>
				</div>
			</TooltipProvider>
		);
	}

	return (
		<TooltipProvider>
			<div className="">
				<div
					className="flex items-center p-3 border-b px-3 justify-between sticky top-0 bg-background z-10"
					data-tauri-drag-region
				>
					<div data-tauri-drag-region>
						<h3 className="font-bold" data-tauri-drag-region>
							{t("configSwitcher.title")}
						</h3>
						<p className="text-sm text-muted-foreground" data-tauri-drag-region>
							{t("configSwitcher.description")}
						</p>
					</div>
					<ButtonGroup>
						<Dialog
							open={isCreateDialogOpen}
							onOpenChange={setIsCreateDialogOpen}
						>
							<DialogTrigger asChild>
								<Button
									variant="outline"
									onClick={openCreateDialog}
									className="text-muted-foreground"
									size="sm"
								>
									<PlusIcon size={14} />
									{t("configSwitcher.createConfig")}
								</Button>
							</DialogTrigger>
							<DialogContent>
								<DialogHeader>
									<DialogTitle>{t("workspace.createWorkspace")}</DialogTitle>
									<DialogDescription>
										{t("workspace.createWorkspaceDescription")}
									</DialogDescription>
								</DialogHeader>
								<div className="space-y-4 py-4">
									<div className="space-y-2">
										<Label htmlFor="title2">
											{t("workspace.workspaceName")}
										</Label>
										<Input
											id="title2"
											placeholder={t("configSwitcher.newConfig")}
											value={newWorkspaceTitle}
											onChange={(e) => setNewWorkspaceTitle(e.target.value)}
										/>
									</div>

									<div className="flex items-center justify-between">
										<div className="flex items-center gap-2">
											<Label htmlFor="fullDirectory2">
												{t("workspace.fullDirectory")}
											</Label>
											<Tooltip>
												<TooltipTrigger>
													<InfoIcon className="h-4 w-4 text-muted-foreground" />
												</TooltipTrigger>
												<TooltipContent className="max-w-xs">
													{t("workspace.fullDirectoryTooltip")}
												</TooltipContent>
											</Tooltip>
										</div>
										<Switch
											id="fullDirectory2"
											checked={isFullDirectory}
											onCheckedChange={setIsFullDirectory}
										/>
									</div>

									{isFullDirectory && (
										<div className="flex items-center justify-between pl-4">
											<div className="flex items-center gap-2">
												<Label htmlFor="includeScripts2">
													{t("workspace.includeScripts")}
												</Label>
												<Tooltip>
													<TooltipTrigger>
														<InfoIcon className="h-4 w-4 text-muted-foreground" />
													</TooltipTrigger>
													<TooltipContent className="max-w-xs">
														{t("workspace.includeScriptsTooltip")}
													</TooltipContent>
												</Tooltip>
											</div>
											<Switch
												id="includeScripts2"
												checked={includeScripts}
												onCheckedChange={setIncludeScripts}
											/>
										</div>
									)}
								</div>
								<DialogFooter>
									<Button
										onClick={onCreateStore}
										disabled={createStoreMutation.isPending}
									>
										{createStoreMutation.isPending
											? t("workspace.creating")
											: t("workspace.create")}
									</Button>
								</DialogFooter>
							</DialogContent>
						</Dialog>
						<DropdownMenu>
							<DropdownMenuTrigger asChild>
								<Button
									variant="outline"
									className="text-muted-foreground"
									size="sm"
								>
									<EllipsisVerticalIcon size={14} />
								</Button>
							</DropdownMenuTrigger>
							<DropdownMenuContent align="end">
								<GLMDialog
									trigger={
										<DropdownMenuItem onSelect={(e) => e.preventDefault()}>
											<ZAI />
											{t("glm.useZhipuGlm")}
										</DropdownMenuItem>
									}
								/>
								<MiniMaxDialog
									trigger={
										<DropdownMenuItem onSelect={(e) => e.preventDefault()}>
											<Minimax />
											{t("minimax.useMiniMax")}
										</DropdownMenuItem>
									}
								/>
								<KimiDialog
									trigger={
										<DropdownMenuItem onSelect={(e) => e.preventDefault()}>
											<Kimi />
											{t("kimi.useKimi")}
										</DropdownMenuItem>
									}
								/>
							</DropdownMenuContent>
						</DropdownMenu>
					</ButtonGroup>
				</div>

				<div className="grid grid-cols-3 lg:grid-cols-4 gap-3 p-4">
					{/* Fixed Claude Original Config Item */}
					<div
						role="button"
						onClick={handleOriginalConfigClick}
						className={cn(
							"border rounded-xl p-3 h-[120px] flex flex-col justify-between transition-colors",
							{
								"bg-primary/10 border-primary border-2": isOriginalConfigActive,
							},
						)}
					>
						<div>
							<div>{t("configSwitcher.originalConfig")}</div>
							<div className="text-xs text-muted-foreground mt-1">
								{t("configSwitcher.originalConfigDescription")}
							</div>
						</div>
					</div>

					{stores.map((store) => {
						const isCurrentStore = store.using;
						const isFullDir = store.workspaceType === "full_directory";
						return (
							<div
								role="button"
								key={store.id}
								onClick={() => handleStoreClick(store.id, isCurrentStore)}
								className={cn(
									"border rounded-xl p-3 h-[120px] flex flex-col justify-between transition-colors disabled:opacity-50",
									{
										"bg-primary/10 border-primary border-2": isCurrentStore,
									},
								)}
							>
								<div>
									<div className="flex items-center gap-2">
										<span className="truncate">{store.title}</span>
										{isFullDir ? (
											<Badge
												variant="secondary"
												className="text-[10px] px-1.5 py-0"
											>
												<FolderIcon className="h-3 w-3 mr-1" />
												{t("workspace.fullDir")}
											</Badge>
										) : (
											<Badge
												variant="outline"
												className="text-[10px] px-1.5 py-0"
											>
												<SettingsIcon className="h-3 w-3 mr-1" />
												{t("workspace.settingsOnly")}
											</Badge>
										)}
									</div>

									{isFullDir && (
										<div className="text-[10px] text-muted-foreground mt-1 flex gap-2">
											<span>
												{store.skillsCount ?? 0} {t("workspace.skills")}
											</span>
											<span>
												{store.commandsCount ?? 0} {t("workspace.commands")}
											</span>
											<span>
												{store.agentsCount ?? 0} {t("workspace.agents")}
											</span>
											<span>
												{store.pluginsCount ?? 0} {t("workspace.plugins")}
											</span>
										</div>
									)}

									{!isFullDir && store.settings.env?.ANTHROPIC_BASE_URL && (
										<div
											className="text-xs text-muted-foreground mt-1 truncate"
											title={store.settings.env.ANTHROPIC_BASE_URL}
										>
											{store.settings.env.ANTHROPIC_BASE_URL}
										</div>
									)}
								</div>

								<div className="flex justify-end gap-1">
									{isFullDir && isCurrentStore && (
										<Tooltip>
											<TooltipTrigger asChild>
												<button
													className="hover:bg-primary/10 rounded-lg p-2 hover:text-primary"
													onClick={(e) => handleSyncWorkspace(e, store.id)}
													disabled={syncWorkspaceMutation.isPending}
												>
													<RefreshCwIcon
														className={cn("text-muted-foreground", {
															"animate-spin": syncWorkspaceMutation.isPending,
														})}
														size={14}
													/>
												</button>
											</TooltipTrigger>
											<TooltipContent>
												{t("workspace.syncTooltip")}
											</TooltipContent>
										</Tooltip>
									)}
									<button
										className="hover:bg-primary/10 rounded-lg p-2 hover:text-primary"
										onClick={(e) => {
											e.stopPropagation();
											navigate(`/edit/${store.id}`);
										}}
									>
										<PencilLineIcon
											className="text-muted-foreground"
											size={14}
										/>
									</button>
								</div>
							</div>
						);
					})}
				</div>

				{/* Confirmation Dialog for Workspace Switching */}
				<Dialog open={isSwitchDialogOpen} onOpenChange={setIsSwitchDialogOpen}>
					<DialogContent className="border border-primary">
						<DialogHeader>
							<DialogTitle className="flex items-center gap-2">
								<AlertTriangleIcon className="h-5 w-5 text-amber-500" />
								{t("workspace.switchWarningTitle")}
							</DialogTitle>
							<DialogDescription className="text-left">
								{t("workspace.switchWarningDescription")}
							</DialogDescription>
						</DialogHeader>
						<p className="text-sm font-medium">
							{t("workspace.switchWarningAction")}
						</p>
						<DialogFooter className="gap-2 mt-4">
							<Button variant="outline" onClick={cancelSwitch}>
								{t("workspace.cancel")}
							</Button>
							<Button
								onClick={confirmSwitch}
								disabled={setCurrentStoreMutation.isPending || resetToOriginalMutation.isPending}
							>
								{(setCurrentStoreMutation.isPending || resetToOriginalMutation.isPending)
									? t("workspace.switching")
									: t("workspace.confirmSwitch")}
							</Button>
						</DialogFooter>
					</DialogContent>
				</Dialog>
			</div>
		</TooltipProvider>
	);
}
