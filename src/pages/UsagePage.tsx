import { useProjectUsageFiles } from "@/lib/query";
import { TokenUsageChart } from "@/components/TokenUsageChart";
import { useState, useEffect } from "react";
import { ProjectUsageRecord } from "@/lib/query";
import { cn, formatLargeNumber } from "@/lib/utils";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { ArrowDownIcon, ArrowUpIcon, CircleDotDashedIcon, RefreshCwIcon } from "lucide-react";

export function UsagePage() {
  const { t } = useTranslation();
  const { data: usageData, isLoading, error, refetch, isRefetching } = useProjectUsageFiles();
  const [filteredUsageData, setFilteredUsageData] = useState<ProjectUsageRecord[]>([]);

  // Initialize filtered data with full data
  useEffect(() => {
    if (usageData) {
      setFilteredUsageData(usageData);
    }
  }, [usageData]);

  return (
    <div className="">
      <div className="flex items-center p-3 border-b px-3 justify-between sticky top-0 bg-background z-10 mb-4" data-tauri-drag-region>
        <div data-tauri-drag-region>
          <h3 className="font-bold" data-tauri-drag-region>{t("usage.title")}</h3>
          <p className="text-sm text-muted-foreground" data-tauri-drag-region>
            {t("usage.description")}
          </p>
        </div>

        <div>
          <Button
            disabled={isRefetching || isLoading}
            onClick={_ => {
              refetch();
            }} variant="ghost" size="sm" className="text-muted-foreground">
            <RefreshCwIcon className={cn({
              "animate-spin": isRefetching || isLoading,
            })} />
            {isRefetching || isLoading ? t("usage.refreshing") : t("usage.refresh")}
          </Button>
        </div>
      </div>
      <div className="px-4 space-y-4">
        {isLoading ? (
          <div className="space-y-6">
            <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
              {[1, 2, 3].map((i) => (
                <div key={i} className="bg-card p-4 rounded-lg space-y-2 border">
                  <div className="h-4  rounded animate-pulse"></div>
                  <div className="h-8 rounded animate-pulse"></div>
                </div>
              ))}
            </div>
            <div className="mt-6 bg-card p-6 rounded-lg w-full min-w-0 border">
              <div className="h-64 rounded animate-pulse"></div>
            </div>
          </div>
        ) : error ? (
          <p>{t("usage.error", { error: error.message })}</p>
        ) : usageData && usageData.length > 0 ? (
          <>
            <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
              <div className="bg-blue-50 border-blue-100 text-blue-700 border-2 p-4 rounded-lg space-y-2 dark:bg-blue-950/20 dark:border-blue-900/30 dark:text-blue-300">
                <div className="flex items-center gap-2">
                  <ArrowDownIcon size={12} />
                  <h3 className="font-medium">{t("usage.inputTokens")}</h3>
                </div>
                <p className="text-2xl font-bold">
                  {formatLargeNumber(filteredUsageData.reduce((sum, record) => sum + (record.usage?.input_tokens || 0), 0))}
                </p>
              </div>
              <div className="bg-emerald-50 border-emerald-100 text-emerald-700 border-2 p-4 rounded-lg space-y-2 dark:bg-emerald-950/20 dark:border-emerald-900/30 dark:text-emerald-300">
                <div className="flex items-center gap-2">
                  <ArrowUpIcon size={12} />
                  <h3 className="font-medium">{t("usage.outputTokens")}</h3>
                </div>
                <p className="text-2xl font-bold">
                  {formatLargeNumber(filteredUsageData.reduce((sum, record) => sum + (record.usage?.output_tokens || 0), 0))}
                </p>
              </div>
              <div className="bg-amber-50 border-amber-100 text-amber-700 border-2 p-4 rounded-lg space-y-2 dark:bg-amber-950/20 dark:border-amber-900/30 dark:text-amber-300">
                <div className="flex items-center gap-2">
                  <CircleDotDashedIcon size={12} />
                  <h3 className="font-medium">{t("usage.cacheReadTokens")}</h3>
                </div>
                <p className="text-2xl font-bold">
                  {formatLargeNumber(filteredUsageData.reduce((sum, record) => sum + (record.usage?.cache_read_input_tokens || 0), 0))}
                </p>
              </div>

              {/* <div className="bg-zinc-50 p-4 rounded-lg space-y-2">
                <div className="flex items-start justify-between">
                  <h3 className="font-medium">Cost</h3>
                  <Button variant="ghost" size="icon" className="-my-2 -mr-2">
                    <SettingsIcon />
                  </Button>
                </div>
                <p className="text-2xl font-bold">
                  $0.00
                </p>
              </div> */}
            </div>

            <div className=" bg-card p-6 rounded-lg w-full min-w-0 border">
              <TokenUsageChart data={usageData} onFilteredDataChange={setFilteredUsageData} />
            </div>
          </>
        ) : (
          <p>{t("usage.noData")}</p>
        )}
      </div>
    </div>
  );
}