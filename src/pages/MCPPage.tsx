import React from "react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { PlusIcon } from "lucide-react";

export function MCPPage() {
  const { t } = useTranslation();

  return (
    <div className="">
      <div className="flex items-center p-3 border-b px-3 justify-between sticky top-0 bg-background z-10 mb-4" data-tauri-drag-region>
        <div data-tauri-drag-region>
          <h3 className="font-bold" data-tauri-drag-region>MCP</h3>
          <p className="text-sm text-muted-foreground" data-tauri-drag-region>
            Claude Code 全局 MCP 服务配置
          </p>
        </div>
        <Button variant="ghost" className="text-muted-foreground" size="sm">
          <PlusIcon size={14} />
          添加 MCP 服务
        </Button>
      </div>
      <div className="space-y-4">
      </div>
    </div>
  );
}