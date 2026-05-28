import type { ComponentPropsWithoutRef, ReactNode } from 'react';
import type { LucideIcon } from 'lucide-react';
import { cn } from '@/lib/utils';

export function ScreenShell({
  children,
  className,
}: {
  children: ReactNode;
  className?: string;
}) {
  return (
    <div
      className={cn(
        'mx-auto flex min-h-full w-full max-w-5xl items-start justify-center px-3 py-3 sm:px-4 sm:py-4 md:px-6 md:py-8',
        className,
      )}
    >
      {children}
    </div>
  );
}

export function StoryFrame({
  children,
  className,
}: {
  children: ReactNode;
  className?: string;
}) {
  return <div className={cn('akashic-surface w-full', className)}>{children}</div>;
}

export function SectionCard({
  children,
  className,
  ...props
}: ComponentPropsWithoutRef<'section'>) {
  return (
    <section className={cn('akashic-panel p-4 sm:p-5 md:p-6', className)} {...props}>
      {children}
    </section>
  );
}

export function StatusPill({
  icon: Icon,
  iconClassName,
  children,
  className,
}: {
  icon: LucideIcon | null;
  iconClassName?: string;
  children: ReactNode;
  className?: string;
}) {
  return (
    <div className={cn('akashic-pill', className)}>
      {Icon ? <Icon className={cn('h-4 w-4 shrink-0', iconClassName)} /> : null}
      <span>{children}</span>
    </div>
  );
}

export function PageTitle({
  title,
  subtitle,
  action,
}: {
  title: string;
  subtitle?: string;
  action?: ReactNode;
}) {
  return (
    <div className="flex flex-col gap-3 md:flex-row md:items-center md:justify-center">
      <div className="space-y-2">
        <h1 className="text-[2rem] font-semibold text-center tracking-wide text-[#f6eddc] sm:text-[2.35rem] md:text-5xl">{title}</h1>
        {subtitle ? <p className="max-w-2xl text-center text-sm leading-6 text-[#9ca7be] sm:text-[0.95rem] md:text-base">{subtitle}</p> : null}
      </div>
      {action ? <div className="shrink-0 self-start">{action}</div> : null}
    </div>
  );
}

export function FieldLabel({
  children,
  hint,
}: {
  children: ReactNode;
  hint?: ReactNode;
}) {
  return (
    <div className="mb-3 flex items-center justify-between gap-3 text-sm font-medium text-[#efe4cd]">
      <span>{children}</span>
      {hint ? <span className="text-xs font-normal text-[#9ca7be]">{hint}</span> : null}
    </div>
  );
}

export function PrimaryButton({
  className,
  ...props
}: ComponentPropsWithoutRef<'button'>) {
  return <button className={cn('akashic-primary-btn', className)} {...props} />;
}

export function SecondaryButton({
  className,
  ...props
}: ComponentPropsWithoutRef<'button'>) {
  return <button className={cn('akashic-secondary-btn', className)} {...props} />;
}
