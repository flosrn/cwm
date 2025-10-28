import { useEffect } from "react";
import { useNavigate } from "react-router-dom";
import { useClaudeProjects } from "../../lib/query";

export function List() {
	const navigate = useNavigate();
	const { data: projects } = useClaudeProjects();

	useEffect(() => {
		if (projects && projects.length > 0) {
			// Navigate to the first project, URL-encoding the path
			const firstProjectPath = encodeURIComponent(projects[0].path);
			navigate(`/projects/${firstProjectPath}`, { replace: true });
		}
	}, [projects, navigate]);

	// This component is mainly used for redirection now
	// The loading, error, and empty states are handled by the layout
	return null;
}
