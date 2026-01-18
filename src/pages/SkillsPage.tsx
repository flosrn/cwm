import { markdown, markdownLanguage } from "@codemirror/lang-markdown";
import { yamlFrontmatter } from "@codemirror/lang-yaml";
import { ask, message } from "@tauri-apps/plugin-dialog";
import CodeMirror, { EditorView } from "@uiw/react-codemirror";
import { FileTextIcon, PlusIcon, SaveIcon, SparklesIcon, TrashIcon } from "lucide-react";
import { Suspense, useState } from "react";
import { useTranslation } from "react-i18next";
import {
	Accordion,
	AccordionContent,
	AccordionItem,
	AccordionTrigger,
} from "@/components/ui/accordion";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogHeader,
	DialogTitle,
	DialogTrigger,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { ScrollArea } from "@/components/ui/scroll-area";
import {
	useClaudeSkills,
	useDeleteClaudeSkill,
	useWriteClaudeSkill,
} from "@/lib/query";
import { useCodeMirrorTheme } from "@/lib/use-codemirror-theme";

function SkillsPageContent() {
	const { t } = useTranslation();
	const { data: skills, isLoading, error } = useClaudeSkills();
	const writeSkill = useWriteClaudeSkill();
	const deleteSkill = useDeleteClaudeSkill();
	const [skillEdits, setSkillEdits] = useState<Record<string, string>>({});
	const [isDialogOpen, setIsDialogOpen] = useState(false);
	const codeMirrorTheme = useCodeMirrorTheme();

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
					{t("skills.error", { error: error.message })}
				</div>
			</div>
		);
	}

	const handleContentChange = (skillName: string, content: string) => {
		setSkillEdits((prev) => ({
			...prev,
			[skillName]: content,
		}));
	};

	const handleSaveSkill = async (skillName: string) => {
		const content = skillEdits[skillName];
		if (content === undefined) return;

		writeSkill.mutate({
			skillName,
			content,
		});
	};

	const handleDeleteSkill = async (skillName: string) => {
		const confirmed = await ask(t("skills.deleteConfirm", { skillName }), {
			title: t("skills.deleteTitle"),
			kind: "warning",
		});

		if (confirmed) {
			deleteSkill.mutate(skillName);
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
						{t("skills.title")}
					</h3>
					<p className="text-sm text-muted-foreground" data-tauri-drag-region>
						{t("skills.description")}
					</p>
				</div>
				<Dialog open={isDialogOpen} onOpenChange={setIsDialogOpen}>
					<DialogTrigger asChild>
						<Button variant="ghost" className="text-muted-foreground" size="sm">
							<PlusIcon size={14} />
							{t("skills.addSkill")}
						</Button>
					</DialogTrigger>
					<DialogContent className="max-w-[600px]">
						<DialogHeader>
							<DialogTitle className="">
								{t("skills.addSkillTitle")}
							</DialogTitle>
							<DialogDescription className="text-muted-foreground text-sm">
								{t("skills.addSkillDescription")}
							</DialogDescription>
						</DialogHeader>
						<CreateSkillPanel onClose={() => setIsDialogOpen(false)} />
					</DialogContent>
				</Dialog>
			</div>
			<div className="">
				{!skills || skills.length === 0 ? (
					<div className="text-center text-muted-foreground py-8">
						{t("skills.noSkills")}
					</div>
				) : (
					<ScrollArea className="h-full">
						<div className="">
							<Accordion type="multiple" className="">
								{skills.map((skill) => (
									<AccordionItem
										key={skill.name}
										value={skill.name}
										className="bg-card"
									>
										<AccordionTrigger className="hover:no-underline px-4 py-2 bg-card hover:bg-accent duration-150">
											<div className="flex items-center gap-2">
												<SparklesIcon size={12} />
												<span className="font-medium">{skill.name}</span>
												{skill.references_count > 0 && (
													<Badge variant="secondary" className="text-[10px] px-1.5 py-0">
														<FileTextIcon className="h-3 w-3 mr-1" />
														{skill.references_count} refs
													</Badge>
												)}
												<span className="text-sm text-muted-foreground font-normal">
													{`~/.claude/skills/${skill.name}/SKILL.md`}
												</span>
											</div>
										</AccordionTrigger>
										<AccordionContent className="pb-3">
											<div className="px-3 pt-3 space-y-3">
												<div className="rounded-lg overflow-hidden border">
													<CodeMirror
														value={
															skillEdits[skill.name] !== undefined
																? skillEdits[skill.name]
																: skill.content
														}
														height="180px"
														theme={codeMirrorTheme}
														onChange={(value) =>
															handleContentChange(skill.name, value)
														}
														placeholder={t("skills.contentPlaceholder")}
														extensions={[
															yamlFrontmatter({
																content: markdown({
																	base: markdownLanguage,
																}),
															}),
															EditorView.lineWrapping,
														]}
														basicSetup={{
															lineNumbers: false,
															highlightActiveLineGutter: true,
															foldGutter: false,
															dropCursor: false,
															allowMultipleSelections: false,
															indentOnInput: true,
															bracketMatching: true,
															closeBrackets: true,
															autocompletion: true,
															highlightActiveLine: true,
															highlightSelectionMatches: true,
															searchKeymap: false,
														}}
													/>
												</div>
												<div className="flex justify-between bg-card">
													<Button
														variant="outline"
														onClick={() => handleSaveSkill(skill.name)}
														disabled={
															writeSkill.isPending ||
															skillEdits[skill.name] === undefined
														}
														size="sm"
													>
														<SaveIcon size={14} className="" />
														{writeSkill.isPending
															? t("skills.saving")
															: t("skills.save")}
													</Button>

													<Button
														variant="ghost"
														size="sm"
														onClick={() => handleDeleteSkill(skill.name)}
														disabled={deleteSkill.isPending}
													>
														<TrashIcon size={14} className="" />
													</Button>
												</div>
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

export function SkillsPage() {
	const { t } = useTranslation();

	return (
		<Suspense
			fallback={
				<div className="flex items-center justify-center min-h-screen">
					<div className="text-center">{t("loading")}</div>
				</div>
			}
		>
			<SkillsPageContent />
		</Suspense>
	);
}

function CreateSkillPanel({ onClose }: { onClose?: () => void }) {
	const { t } = useTranslation();
	const [skillName, setSkillName] = useState("");
	const [skillContent, setSkillContent] = useState(`# Skill Name

## Description
Brief description of what this skill does.

## When to Use
- Trigger condition 1
- Trigger condition 2

## Instructions
Detailed instructions for Claude when this skill is activated.

## Examples
\`\`\`
Example usage or code
\`\`\`
`);
	const writeSkill = useWriteClaudeSkill();
	const { data: skills } = useClaudeSkills();
	const codeMirrorTheme = useCodeMirrorTheme();

	const handleCreateSkill = async () => {
		// Validate skill name
		if (!skillName.trim()) {
			await message(t("skills.emptyNameError"), {
				title: t("skills.validationError"),
				kind: "error",
			});
			return;
		}

		// Check if skill already exists
		const exists = skills?.some((skill) => skill.name === skillName);
		if (exists) {
			await message(t("skills.skillExistsError", { skillName }), {
				title: t("skills.skillExistsTitle"),
				kind: "info",
			});
			return;
		}

		// Validate content
		if (!skillContent.trim()) {
			await message(t("skills.emptyContentError"), {
				title: t("skills.validationError"),
				kind: "error",
			});
			return;
		}

		writeSkill.mutate(
			{
				skillName,
				content: skillContent,
			},
			{
				onSuccess: () => {
					setSkillName("");
					setSkillContent("");
					onClose?.();
				},
			},
		);
	};

	return (
		<div className="space-y-4 mt-4">
			<div className="space-y-2">
				<Label className="block" htmlFor="skill-name">
					{t("skills.skillName")}
				</Label>
				<Input
					id="skill-name"
					value={skillName}
					onChange={(e) => setSkillName(e.target.value)}
					placeholder={t("skills.skillNamePlaceholder")}
				/>
			</div>

			<div className="space-y-2">
				<Label className="block" htmlFor="skill-content">
					{t("skills.skillContent")}
				</Label>
				<div className="rounded-lg overflow-hidden border">
					<CodeMirror
						value={skillContent}
						onChange={(value) => setSkillContent(value)}
						height="200px"
						theme={codeMirrorTheme}
						placeholder={t("skills.contentPlaceholder")}
						extensions={[
							yamlFrontmatter({
								content: markdown({
									base: markdownLanguage,
								}),
							}),
							EditorView.lineWrapping,
						]}
						basicSetup={{
							lineNumbers: false,
							highlightActiveLineGutter: true,
							foldGutter: false,
							dropCursor: false,
							allowMultipleSelections: false,
							indentOnInput: true,
							bracketMatching: true,
							closeBrackets: true,
							autocompletion: true,
							highlightActiveLine: true,
							highlightSelectionMatches: true,
							searchKeymap: false,
						}}
					/>
				</div>
			</div>

			<div className="flex justify-end">
				<Button
					onClick={handleCreateSkill}
					disabled={
						!skillName.trim() || !skillContent.trim() || writeSkill.isPending
					}
				>
					{writeSkill.isPending ? t("skills.creating") : t("skills.create")}
				</Button>
			</div>
		</div>
	);
}
