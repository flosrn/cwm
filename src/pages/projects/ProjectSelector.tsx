import { ChevronDownIcon, FolderIcon } from "lucide-react";
import { useLocation, useNavigate } from "react-router-dom";
import { Button } from "../../components/ui/button";
import {
	DropdownMenu,
	DropdownMenuContent,
	DropdownMenuItem,
	DropdownMenuTrigger,
} from "../../components/ui/dropdown-menu";
import type { ProjectConfig } from "../../lib/query";

interface ProjectSelectorProps {
	projects: ProjectConfig[];
}

export function ProjectSelector({ projects }: ProjectSelectorProps) {
	const navigate = useNavigate();
	const location = useLocation();

	// Get the current project path from the URL
	const currentProjectPath = location.pathname.startsWith("/projects/")
		? decodeURIComponent(location.pathname.replace("/projects/", ""))
		: null;

	// Find the current project
	const currentProject = currentProjectPath
		? projects.find((p) => p.path === currentProjectPath)
		: null;

	const handleProjectSelect = (projectPath: string) => {
		const encodedPath = encodeURIComponent(projectPath);
		navigate(`/projects/${encodedPath}`);
	};

	return (
		<div
			className="flex items-center p-3 border-b bg-background sticky top-0 z-10"
			data-tauri-drag-region
		>
			<div className="flex items-center gap-2 flex-1" data-tauri-drag-region>
				<h3 className="font-bold" data-tauri-drag-region>
					Projects
				</h3>
			</div>
			<DropdownMenu>
				<DropdownMenuTrigger asChild>
					<Button
						variant="outline"
						className="flex items-center gap-2 min-w-[200px] justify-between"
					>
						<div className="flex items-center gap-2 truncate">
							<FolderIcon className="h-4 w-4 flex-shrink-0" />
							<span className="truncate">
								{currentProject
									? currentProject.path.split("/").pop() || currentProject.path
									: "Select Project"}
							</span>
						</div>
						<ChevronDownIcon className="h-4 w-4 flex-shrink-0" />
					</Button>
				</DropdownMenuTrigger>
				<DropdownMenuContent
					align="end"
					className="min-w-[250px] max-h-[300px] overflow-y-auto"
				>
					{projects.map((project) => (
						<DropdownMenuItem
							key={project.path}
							onClick={() => handleProjectSelect(project.path)}
							className="flex items-center gap-2 cursor-pointer"
						>
							<FolderIcon className="h-4 w-4" />
							<div className="flex flex-col truncate">
								<span className="truncate font-medium">
									{project.path.split("/").pop() || project.path}
								</span>
								<span className="text-xs text-muted-foreground truncate">
									{project.path}
								</span>
							</div>
						</DropdownMenuItem>
					))}
				</DropdownMenuContent>
			</DropdownMenu>
		</div>
	);
}
