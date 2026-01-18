import { ask } from "@tauri-apps/plugin-dialog";
import {
	BotIcon,
	CpuIcon,
	PackageIcon,
	SparklesIcon,
	TerminalIcon,
	TrashIcon,
} from "lucide-react";
import { Suspense } from "react";
import { useTranslation } from "react-i18next";
import {
	Accordion,
	AccordionContent,
	AccordionItem,
	AccordionTrigger,
} from "@/components/ui/accordion";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Switch } from "@/components/ui/switch";
import {
	useClaudePlugins,
	useDeleteLocalPlugin,
	useTogglePlugin,
} from "@/lib/query";

function PluginsPageContent() {
	const { t } = useTranslation();
	const { data: plugins, isLoading, error } = useClaudePlugins();
	const togglePlugin = useTogglePlugin();
	const deletePlugin = useDeleteLocalPlugin();

	if (isLoading) {
		return (
			<div className="flex items-center justify-center min-h-screen">
				<div className="text-center">{t("loading")}</div>
			</div>
		);
	}

	if (error) {
		return (
			<div className="flex items-center justify-center min-h-screen">
				<div className="text-center text-red-500">
					{t("plugins.error", { error: error.message })}
				</div>
			</div>
		);
	}

	const handleTogglePlugin = async (pluginPath: string, enabled: boolean) => {
		togglePlugin.mutate({ pluginPath, enabled });
	};

	const handleDeletePlugin = async (pluginName: string) => {
		const confirmed = await ask(t("plugins.deleteConfirm", { pluginName }), {
			title: t("plugins.deleteTitle"),
			kind: "warning",
		});

		if (confirmed) {
			deletePlugin.mutate(pluginName);
		}
	};

	return (
		<div className="">
			<div
				className="flex items-center p-3 border-b px-3 justify-between sticky top-0 bg-background z-10"
				data-tauri-drag-region
			>
				<div data-tauri-drag-region>
					<h3 className="font-bold" data-tauri-drag-region>
						{t("plugins.title")}
					</h3>
					<p className="text-sm text-muted-foreground" data-tauri-drag-region>
						{t("plugins.description")}
					</p>
				</div>
			</div>
			<div className="">
				{!plugins || plugins.length === 0 ? (
					<div className="text-center text-muted-foreground py-8">
						{t("plugins.noPlugins")}
					</div>
				) : (
					<ScrollArea className="h-full">
						<div className="">
							<Accordion type="multiple" className="">
								{plugins.map((plugin) => (
									<AccordionItem
										key={plugin.path}
										value={plugin.path}
										className="bg-card"
									>
										<AccordionTrigger className="hover:no-underline px-4 py-2 bg-card hover:bg-accent duration-150">
											<div className="flex items-center gap-2 flex-1">
												<PackageIcon size={12} />
												<span className="font-medium">{plugin.name}</span>
												<Badge
													variant={plugin.is_local ? "default" : "secondary"}
													className="text-[10px] px-1.5 py-0"
												>
													{plugin.is_local
														? t("plugins.local")
														: t("plugins.marketplace")}
												</Badge>
												{plugin.has_mcp && (
													<Badge
														variant="outline"
														className="text-[10px] px-1.5 py-0"
													>
														<CpuIcon className="h-3 w-3 mr-1" />
														MCP
													</Badge>
												)}
											</div>
											<div
												className="flex items-center gap-2 mr-2"
												onClick={(e) => e.stopPropagation()}
											>
												<Switch
													checked={plugin.is_enabled}
													onCheckedChange={(checked) =>
														handleTogglePlugin(plugin.path, checked)
													}
													disabled={togglePlugin.isPending}
												/>
											</div>
										</AccordionTrigger>
										<AccordionContent className="pb-3">
											<div className="px-4 pt-3 space-y-3">
												{plugin.description && (
													<p className="text-sm text-muted-foreground">
														{plugin.description}
													</p>
												)}

												<div className="flex flex-wrap gap-2">
													{plugin.skills_count > 0 && (
														<Badge
															variant="secondary"
															className="text-[10px] px-1.5 py-0"
														>
															<SparklesIcon className="h-3 w-3 mr-1" />
															{plugin.skills_count}{" "}
															{plugin.skills_count === 1
																? t("plugins.skill")
																: t("plugins.skills")}
														</Badge>
													)}
													{plugin.commands_count > 0 && (
														<Badge
															variant="secondary"
															className="text-[10px] px-1.5 py-0"
														>
															<TerminalIcon className="h-3 w-3 mr-1" />
															{plugin.commands_count}{" "}
															{plugin.commands_count === 1
																? t("plugins.command")
																: t("plugins.commands")}
														</Badge>
													)}
													{plugin.agents_count > 0 && (
														<Badge
															variant="secondary"
															className="text-[10px] px-1.5 py-0"
														>
															<BotIcon className="h-3 w-3 mr-1" />
															{plugin.agents_count}{" "}
															{plugin.agents_count === 1
																? t("plugins.agent")
																: t("plugins.agents")}
														</Badge>
													)}
												</div>

												<p className="text-xs text-muted-foreground font-mono">
													{plugin.path}
												</p>

												{plugin.is_local && (
													<div className="flex justify-end">
														<Button
															variant="ghost"
															size="sm"
															onClick={() => handleDeletePlugin(plugin.name)}
															disabled={deletePlugin.isPending}
															className="text-destructive hover:text-destructive"
														>
															<TrashIcon size={14} className="mr-1" />
															{t("plugins.delete")}
														</Button>
													</div>
												)}
											</div>
										</AccordionContent>
									</AccordionItem>
								))}
							</Accordion>
						</div>
					</ScrollArea>
				)}
			</div>
		</div>
	);
}

export function PluginsPage() {
	const { t } = useTranslation();

	return (
		<Suspense
			fallback={
				<div className="flex items-center justify-center min-h-screen">
					<div className="text-center">{t("loading")}</div>
				</div>
			}
		>
			<PluginsPageContent />
		</Suspense>
	);
}
