import { FolderOpenIcon, Loader2 } from "lucide-react";
import { Outlet } from "react-router-dom";
import { Alert, AlertDescription } from "../../components/ui/alert";
import { useClaudeProjects } from "../../lib/query";

export function ProjectsLayout() {
	const { data: projects, isLoading, error } = useClaudeProjects();

	if (isLoading) {
		return (
			<div className="flex flex-col h-full">
				<div className="flex items-center justify-center py-8">
					<Loader2 className="h-6 w-6 animate-spin" />
					<span className="ml-2 text-sm text-muted-foreground">
						Loading projects...
					</span>
				</div>
			</div>
		);
	}

	if (error) {
		return (
			<div className="flex flex-col h-full">
				<Alert>
					<AlertDescription>
						Failed to load projects:{" "}
						{error instanceof Error ? error.message : String(error)}
					</AlertDescription>
				</Alert>
			</div>
		);
	}

	if (!projects || projects.length === 0) {
		return (
			<div className="flex flex-col h-full">
				<div className="flex flex-col items-center justify-center py-16 text-center">
					<FolderOpenIcon className="h-12 w-12 text-muted-foreground mb-4" />
					<h3 className="text-lg font-semibold mb-2">No Projects Found</h3>
					<p className="text-sm text-muted-foreground max-w-md">
						There are no Claude projects configured. Projects appear here when
						you use Claude Code in different project folders.
					</p>
				</div>
			</div>
		);
	}

	return (
		<div className="flex flex-col h-full">
			{/* <ProjectSelector projects={projects} /> */}
			<Outlet />
		</div>
	);
}
