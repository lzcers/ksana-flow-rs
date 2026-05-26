import type { ChangeEvent, HTMLInputTypeAttribute, ReactNode, SyntheticEvent } from 'react';
import { type NodeProps } from '@xyflow/react';
import type { NodeData } from '@/model/workflow/types';
import { cn } from '@/utils/cn';
import { NodeWrapper } from './NodeWrapper';
import { nodeFormStyles } from './formStyles';

type FormOption = {
  value: string;
  label: string;
};

type BaseField = {
  label: string;
  value: string;
  onChange: (next: string) => void;
  placeholder?: string;
  controlVariant?: 'default' | 'plain';
  controlClassName?: string;
  fieldClassName?: string;
  labelClassName?: string;
  hint?: string;
  hintClassName?: string;
  onBlur?: () => void;
  onCompositionStart?: () => void;
  onCompositionEnd?: (next: string) => void;
};

type InputField = BaseField & {
  kind: 'input';
  inputType?: HTMLInputTypeAttribute;
  min?: number | string;
  max?: number | string;
  step?: number | string;
};

type TextareaField = BaseField & {
  kind: 'textarea';
  rows?: number;
};

type SelectField = BaseField & {
  kind: 'select';
  options: FormOption[];
};

type CheckboxField = {
  kind: 'checkbox';
  label: string;
  checked: boolean;
  onChange: (next: boolean) => void;
  controlClassName?: string;
  fieldClassName?: string;
  labelClassName?: string;
  hint?: string;
  hintClassName?: string;
};

export type FormField = InputField | TextareaField | SelectField | CheckboxField;

export type FormGroup = {
  fields: FormField[];
  layout?: 'stack' | 'grid2';
  className?: string;
  title?: string;
  titleClassName?: string;
};

type FormNodeShellProps = Pick<NodeProps, 'id' | 'type' | 'selected' | 'width' | 'height'>;

function stopNodeEvent(event: SyntheticEvent) {
  event.stopPropagation();
}

function getFieldControlClassName(field: FormField) {
  if (field.kind === 'checkbox') {
    return nodeFormStyles.checkbox;
  }
  if (field.controlVariant === 'plain') {
    if (field.kind === 'select') {
      return nodeFormStyles.selectPlain;
    }
    if (field.kind === 'input') {
      return nodeFormStyles.inputPlain;
    }
  }
  if (field.kind === 'textarea') {
    return nodeFormStyles.textarea;
  }
  if (field.kind === 'select') {
    return nodeFormStyles.select;
  }
  return nodeFormStyles.input;
}

function renderField(field: FormField, index: number) {
  const key = `${field.kind}-${field.label}-${index}`;

  if (field.kind === 'checkbox') {
    return (
      <div key={key} className={cn(nodeFormStyles.field, field.fieldClassName)}>
        <label className={nodeFormStyles.inlineField}>
          <span className={cn(nodeFormStyles.inlineLabel, field.labelClassName)}>{field.label}</span>
          <input
            type="checkbox"
            checked={field.checked}
            onChange={event => field.onChange(event.target.checked)}
            onKeyDown={stopNodeEvent}
            onMouseDown={stopNodeEvent}
            onPointerDown={stopNodeEvent}
            className={cn(getFieldControlClassName(field), field.controlClassName)}
          />
        </label>
        {field.hint ? <p className={cn(nodeFormStyles.hint, field.hintClassName)}>{field.hint}</p> : null}
      </div>
    );
  }

  const commonProps = {
    value: field.value,
    onChange: (event: ChangeEvent<HTMLInputElement | HTMLTextAreaElement | HTMLSelectElement>) =>
      field.onChange(event.target.value),
    onKeyDown: stopNodeEvent,
    onMouseDown: stopNodeEvent,
    onPointerDown: stopNodeEvent,
    onBlur: field.onBlur,
    className: cn(getFieldControlClassName(field), field.controlClassName),
  };

  return (
    <div key={key} className={cn(nodeFormStyles.field, field.fieldClassName)}>
      <label className={cn(nodeFormStyles.label, field.labelClassName)}>{field.label}</label>

      {field.kind === 'textarea' ? (
        <textarea
          {...commonProps}
          rows={field.rows ?? 4}
          onCompositionStart={field.onCompositionStart}
          onCompositionEnd={event => field.onCompositionEnd?.(event.currentTarget.value)}
          placeholder={field.placeholder}
          spellCheck={false}
        />
      ) : field.kind === 'select' ? (
        <select {...commonProps}>
          {field.options.map(option => (
            <option key={option.value} value={option.value}>
              {option.label}
            </option>
          ))}
        </select>
      ) : (
        <input
          {...commonProps}
          type={field.inputType}
          min={field.min}
          max={field.max}
          step={field.step}
          onCompositionStart={field.onCompositionStart}
          onCompositionEnd={event => field.onCompositionEnd?.(event.currentTarget.value)}
          placeholder={field.placeholder}
          spellCheck={false}
        />
      )}

      {field.hint ? <p className={cn(nodeFormStyles.hint, field.hintClassName)}>{field.hint}</p> : null}
    </div>
  );
}

export function FormNodeView({
  id,
  type,
  data,
  selected,
  width,
  height,
  minWidth,
  minHeight,
  className,
  contentClassName,
  groups,
  children,
}: FormNodeShellProps & {
  data: NodeData;
  minWidth: number;
  minHeight: number;
  className?: string;
  contentClassName?: string;
  groups?: FormGroup[];
  children?: ReactNode;
}) {
  return (
    <NodeWrapper
      id={id}
      type={type}
      data={data}
      selected={selected}
      className={className}
      minWidth={minWidth}
      minHeight={minHeight}
      style={{ width, height }}
    >
      <div className={cn(nodeFormStyles.section, contentClassName)}>
        {groups?.map((group, groupIndex) => (
          <div
            key={`group-${groupIndex}`}
            className={cn(group.layout === 'grid2' ? nodeFormStyles.grid2 : nodeFormStyles.stack, group.className)}
          >
            {group.title ? <div className={cn(nodeFormStyles.title, group.titleClassName)}>{group.title}</div> : null}
            {group.fields.map(renderField)}
          </div>
        ))}
        {children}
      </div>
    </NodeWrapper>
  );
}
