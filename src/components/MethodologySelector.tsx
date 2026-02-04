import { useState } from "react";
import { useTranslation } from "react-i18next";
import {
  Methodology,
  useMethodologies,
  useSwitchMethodology,
} from "@/lib/query";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { Badge } from "@/components/ui/badge";
import { cn } from "@/lib/utils";
import {
  Rocket,
  Sparkles,
  Zap,
  Check,
  Loader2,
  FolderCode,
  FileText,
  Bot,
} from "lucide-react";

// Map icon names to Lucide components
const iconMap: Record<string, React.ComponentType<{ className?: string }>> = {
  rocket: Rocket,
  sparkles: Sparkles,
  zap: Zap,
};

interface MethodologyCardProps {
  methodology: Methodology;
  onSelect: (id: string) => void;
  isLoading: boolean;
  loadingId: string | null;
}

function MethodologyCard({
  methodology,
  onSelect,
  isLoading,
  loadingId,
}: MethodologyCardProps) {
  const { t } = useTranslation();
  const IconComponent = methodology.icon
    ? iconMap[methodology.icon] || Rocket
    : Rocket;
  const isCurrentlyLoading = isLoading && loadingId === methodology.id;
  const isDisabled = isLoading || methodology.is_active;

  return (
    <Card
      className={cn(
        "relative cursor-pointer transition-all hover:shadow-md",
        methodology.is_active && "ring-2 ring-primary",
        isDisabled && !methodology.is_active && "opacity-50 cursor-not-allowed"
      )}
      onClick={() => !isDisabled && onSelect(methodology.id)}
    >
      {methodology.is_active && (
        <div className="absolute -top-2 -right-2 z-10">
          <Badge className="bg-primary text-primary-foreground">
            <Check className="w-3 h-3 mr-1" />
            {t("methodology.active")}
          </Badge>
        </div>
      )}
      <CardContent className="p-4">
        <div className="flex items-start gap-3">
          <div
            className="p-2 rounded-lg"
            style={{
              backgroundColor: methodology.color
                ? `${methodology.color}20`
                : "#10b98120",
            }}
          >
            <IconComponent
              className="w-6 h-6"
              style={{ color: methodology.color || "#10b981" }}
            />
          </div>
          <div className="flex-1 min-w-0">
            <h3 className="font-semibold text-sm truncate">{methodology.name}</h3>
            <p className="text-xs text-muted-foreground line-clamp-2 mt-1">
              {methodology.description}
            </p>
          </div>
        </div>

        <div className="flex items-center gap-3 mt-3 text-xs text-muted-foreground">
          <TooltipProvider>
            <Tooltip>
              <TooltipTrigger asChild>
                <div className="flex items-center gap-1">
                  <FolderCode className="w-3 h-3" />
                  <span>{methodology.skills_count}</span>
                </div>
              </TooltipTrigger>
              <TooltipContent>
                {t("methodology.skillsCount", { count: methodology.skills_count })}
              </TooltipContent>
            </Tooltip>
          </TooltipProvider>

          <TooltipProvider>
            <Tooltip>
              <TooltipTrigger asChild>
                <div className="flex items-center gap-1">
                  <FileText className="w-3 h-3" />
                  <span>{methodology.commands_count}</span>
                </div>
              </TooltipTrigger>
              <TooltipContent>
                {t("methodology.commandsCount", { count: methodology.commands_count })}
              </TooltipContent>
            </Tooltip>
          </TooltipProvider>

          <TooltipProvider>
            <Tooltip>
              <TooltipTrigger asChild>
                <div className="flex items-center gap-1">
                  <Bot className="w-3 h-3" />
                  <span>{methodology.agents_count}</span>
                </div>
              </TooltipTrigger>
              <TooltipContent>
                {t("methodology.agentsCount", { count: methodology.agents_count })}
              </TooltipContent>
            </Tooltip>
          </TooltipProvider>
        </div>

        {!methodology.is_active && (
          <Button
            variant="outline"
            size="sm"
            className="w-full mt-3"
            disabled={isDisabled}
            onClick={(e) => {
              e.stopPropagation();
              onSelect(methodology.id);
            }}
          >
            {isCurrentlyLoading ? (
              <>
                <Loader2 className="w-3 h-3 mr-1 animate-spin" />
                {t("methodology.switching")}
              </>
            ) : (
              t("methodology.activate")
            )}
          </Button>
        )}
      </CardContent>
    </Card>
  );
}

export function MethodologySelector() {
  const { t } = useTranslation();
  const { data: methodologies, isLoading: isLoadingMethodologies } = useMethodologies();
  const switchMutation = useSwitchMethodology();
  const [loadingId, setLoadingId] = useState<string | null>(null);

  const handleSelect = async (methodologyId: string) => {
    setLoadingId(methodologyId);
    try {
      await switchMutation.mutateAsync(methodologyId);
    } finally {
      setLoadingId(null);
    }
  };

  if (isLoadingMethodologies) {
    return (
      <div className="flex items-center justify-center p-8">
        <Loader2 className="w-6 h-6 animate-spin text-muted-foreground" />
      </div>
    );
  }

  if (!methodologies || methodologies.length === 0) {
    return (
      <div className="text-center p-8 text-muted-foreground">
        {t("methodology.noMethodologies")}
      </div>
    );
  }

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-lg font-semibold">{t("methodology.title")}</h2>
          <p className="text-sm text-muted-foreground">
            {t("methodology.description")}
          </p>
        </div>
      </div>

      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4">
        {methodologies.map((methodology) => (
          <MethodologyCard
            key={methodology.id}
            methodology={methodology}
            onSelect={handleSelect}
            isLoading={switchMutation.isPending}
            loadingId={loadingId}
          />
        ))}
      </div>
    </div>
  );
}
