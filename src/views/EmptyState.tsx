import { History, type LucideIcon, Package } from "lucide-react";

function Empty({
  icon: Icon,
  title,
  hint,
}: {
  icon: LucideIcon;
  title: string;
  hint: string;
}) {
  return (
    <div className="flex h-full flex-col items-center justify-center gap-3 text-center">
      <span className="flex h-14 w-14 items-center justify-center rounded-2xl bg-secondary text-muted-foreground">
        <Icon className="h-7 w-7" />
      </span>
      <div className="text-sm font-medium text-foreground">{title}</div>
      <div className="max-w-xs text-xs text-muted-foreground">{hint}</div>
    </div>
  );
}

export function RunsView() {
  return (
    <Empty
      icon={History}
      title="暂无运行记录"
      hint="这里将展示工作流的执行历史：耗时、状态、节点日志等。"
    />
  );
}

export function ResourcesView() {
  return (
    <Empty
      icon={Package}
      title="暂无资源"
      hint="这里将管理字典、样本文件、脚本与插件等资源。"
    />
  );
}
