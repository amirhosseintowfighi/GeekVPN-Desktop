import type { ReactNode } from "react";
import type { IconName } from "../design-system/Icon";
import { EmptyState, PageHeader } from "../design-system/layout";

/**
 * A sidebar section before it has anything to show: its header and one honest
 * line on what fills it. Each is replaced by its real screen in later phases.
 */
export function Section({
  title,
  subtitle,
  icon,
  emptyTitle,
  emptyText,
  action,
}: {
  title: string;
  subtitle?: string;
  icon: IconName;
  emptyTitle: string;
  emptyText: string;
  action?: ReactNode;
}) {
  return (
    <div className="flex min-w-0 flex-1 flex-col gap-3.5">
      <PageHeader title={title} subtitle={subtitle} />
      <EmptyState icon={icon} title={emptyTitle} text={emptyText} action={action} />
    </div>
  );
}
