import { openUrl } from "@tauri-apps/plugin-opener"
import { Button } from "./ui/button"
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle, DialogTrigger, } from "./ui/dialog"
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from "./ui/tooltip"
import { ArrowRightIcon, CheckCircleIcon, CircleQuestionMarkIcon, ExternalLinkIcon, InfoIcon } from "lucide-react"
import { Input } from "./ui/input"

export function GLMBanner() {
  return (
    <div className="bg-zinc-50 rounded-md p-2 border border-zinc-200 space-y-2">
      <h3 className="text-zinc-800 text-sm font-medium flex items-center gap-2">在 Claude Code 中使用 GLM 4.6
        <TooltipProvider>
          <Tooltip delayDuration={100}>
            <TooltipTrigger>
              <CircleQuestionMarkIcon size={14} className="text-zinc-500" />
            </TooltipTrigger>
            <TooltipContent className="w-[200px]">
              <p className="font-normal">「在公开基准与真实编程任务中，GLM-4.6 的代码能力对齐 Claude Sonnet 4，是国内已知的最好的 Coding 模型」 —— 智谱官方文档</p>
            </TooltipContent>
          </Tooltip>
        </TooltipProvider>
      </h3>
      <div className="flex items-center gap-1">
        <GLMDialog
          trigger={
            <Button size="sm" variant="outline" className="text-sm">
              开始配置
            </Button>
          }
        />
        <Button size="sm" variant="ghost" className="text-sm">
          关闭
        </Button>
      </div>
    </div>
  )
}

export function GLMDialog(props: {
  trigger: React.ReactNode
}) {
  return (
    <Dialog>
      <DialogTrigger asChild>
        {props.trigger}
      </DialogTrigger>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>
            配置智谱 GLM
          </DialogTitle>
          <DialogDescription>
            在 Claude Code 中使用智谱 GLM
          </DialogDescription>
        </DialogHeader>
        <div className="mt-4">
          <div className="space-y-3">
            <div>
              <h2 className="text-zinc-800 text-sm font-medium flex items-center gap-2">
                第 1 步：购买 GLM 编码畅玩套餐
              </h2>
              <div className="space-y-2 bg-zinc-100 p-3 rounded-lg m-2">
                <Button onClick={_ => {
                  openUrl("https://www.bigmodel.cn/claude-code?ic=UP1VEQEATH")
                }} size="sm" variant="outline" className="text-sm">
                  <ExternalLinkIcon />
                  前往官网购买
                </Button>
                <p className="text-muted-foreground text-sm flex items-center gap-1">
                  使用此按钮购买时，下单立减10%金额（官方活动）</p>
              </div>
            </div>

            <div>
              <h2 className="text-zinc-800 text-sm font-medium flex items-center gap-2">
                第 2 步：创建 API Key
              </h2>
              <div className="space-y-2 bg-zinc-100 p-3 rounded-lg m-2">
                <Button onClick={_ => {
                  openUrl("https://bigmodel.cn/usercenter/proj-mgmt/apikeys")
                }} size="sm" variant="outline" className="text-sm">
                  <ExternalLinkIcon />
                  进入控制台
                </Button>
              </div>
            </div>

            <div>
              <h2 className="text-zinc-800 text-sm font-medium flex items-center gap-2">
                第 3 步：输入 API Key
              </h2>
              <div className="space-y-2 bg-zinc-100 p-3 rounded-lg m-2">
                <Input />
              </div>
            </div>

            <div className="flex justify-end mx-2 mt-2">
              <Button>
                创建配置
              </Button>
            </div>
          </div>
        </div>
      </DialogContent>
    </Dialog>
  )
}