import { CircleQuestionMarkIcon } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { cn } from "@/lib/utils";
import { GLMDialog } from "./GLMDialog";
import { Button } from "./ui/button";
import {
	Tooltip,
	TooltipContent,
	TooltipProvider,
	TooltipTrigger,
} from "./ui/tooltip";

export { GLMDialog } from "./GLMDialog";

export function GLMBanner(props: {
	className?: string;
	hideCloseButton?: boolean;
}) {
	const { t, i18n } = useTranslation();
	const [isDismissed, setIsDismissed] = useState(
		localStorage.getItem("glm-banner-dismissed") === "true",
	);

	const handleDismiss = () => {
		localStorage.setItem("glm-banner-dismissed", "true");
		setIsDismissed(true);
	};

	// Only show banner when locale is Chinese
	if (i18n.language !== "zh" || isDismissed) {
		return null;
	}

	return (
		<div
			className={cn("bg-card rounded-md p-2 border space-y-2", props.className)}
		>
			<h3 className="text-card-foreground text-sm font-medium flex items-center gap-2">
				{t("glm.title")}
				<TooltipProvider>
					<Tooltip delayDuration={100}>
						<TooltipTrigger>
							<CircleQuestionMarkIcon
								size={14}
								className="text-muted-foreground"
							/>
						</TooltipTrigger>
						<TooltipContent className="w-[200px]">
							<p className="font-normal">{t("glm.tooltip")}</p>
						</TooltipContent>
					</Tooltip>
				</TooltipProvider>
			</h3>
			<div className="flex items-center gap-1">
				<GLMDialog
					trigger={
						<Button size="sm" variant="outline" className="text-sm">
							{t("glm.startConfig")}
						</Button>
					}
					onSuccess={handleDismiss}
				/>
				{!props.hideCloseButton && (
					<Button
						size="sm"
						variant="ghost"
						className="text-sm"
						onClick={handleDismiss}
					>
						{t("glm.close")}
					</Button>
				)}
			</div>
		</div>
	);
}
