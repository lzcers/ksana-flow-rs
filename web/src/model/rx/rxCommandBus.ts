/**
 * RxCommandBus - Command 总线
 * 使用 RxJS Subject 分发 Command，由注册的处理器处理
 */

import { Subject, Observable, of, from } from 'rxjs';
import { filter, concatMap, catchError } from 'rxjs/operators';
import type { WorkflowState } from '../types';
import type { GraphCommand } from '../commands';
import { RxWorkflowState } from './rxState';

// 处理器函数类型
export type CommandHandler<T extends GraphCommand> = (
  state: WorkflowState,
  command: T
) => WorkflowState;

export type AsyncCommandHandler<T extends GraphCommand> = (
  state: WorkflowState,
  command: T
) => Promise<WorkflowState>;

export interface RxCommandBusOptions {
  initialState?: WorkflowState;
  enableLogging?: boolean;
  onAfterCommand?: (args: {
    command: GraphCommand;
    prevState: WorkflowState;
    nextState: WorkflowState;
  }) => void;
  onCommandError?: (args: {
    command: GraphCommand;
    error: unknown;
    state: WorkflowState;
  }) => void;
}

export class RxCommandBus {
  private _commands$ = new Subject<GraphCommand>();
  private _state: RxWorkflowState;
  private _handlers = new Map<
    string,
    CommandHandler<any> | AsyncCommandHandler<any>
  >();
  private _options: RxCommandBusOptions;

  // 公共 Observable
  public readonly commands$ = this._commands$.asObservable();
  public get state$(): Observable<WorkflowState> {
    return this._state.state$;
  }

  constructor(options: RxCommandBusOptions = {}) {
    this._options = options;

    const initialState: WorkflowState = options.initialState ?? {
      nodes: [],
      edges: [],
      selectedNodeId: null,
    };

    this._state = new RxWorkflowState({ initialState });

    // 订阅 commands 流
    this._setupCommandProcessing();
  }

  // ===== 公共方法 =====

  /**
   * 分发 Command
   */
  dispatch(command: GraphCommand): void {
    if (this._options.enableLogging) {
      console.log('[RxCommandBus] Dispatch:', command.type, command.payload);
    }
    this._commands$.next(command);
  }

  /**
   * 获取当前状态快照
   */
  get currentState(): WorkflowState {
    return this._state.state;
  }

  /**
   * 直接设置状态（用于初始化或恢复）
   */
  setState(state: WorkflowState): void {
    this._state.next(state);
  }

  /**
   * 注册 Command 处理器
   */
  registerHandler<T extends GraphCommand>(
    type: T['type'],
    handler: CommandHandler<T> | AsyncCommandHandler<T>
  ): void {
    this._handlers.set(type, handler);
  }

  /**
   * 批量注册处理器
   */
  registerHandlers(
    handlers: Record<
      string,
      CommandHandler<any> | AsyncCommandHandler<any>
    >
  ): void {
    Object.entries(handlers).forEach(([type, handler]) => {
      this._handlers.set(type, handler);
    });
  }

  /**
   * 订阅特定类型的 Command
   */
  onCommand<T extends GraphCommand>(type: T['type']): Observable<T> {
    return this._commands$.pipe(
      filter((cmd): cmd is T => cmd.type === type)
    );
  }

  /**
   * 订阅状态变化
   */
  onStateChange(): Observable<WorkflowState> {
    return this._state.state$;
  }

  /**
   * 选择特定节点
   */
  selectNode$(nodeId: string) {
    return this._state.selectNode$(nodeId);
  }

  /**
   * 销毁（清理订阅）
   */
  destroy(): void {
    this._commands$.complete();
    this._state.destroy();
  }

  // ===== 私有方法 =====

  /**
   * 设置 Command 处理流程
   */
  private _setupCommandProcessing(): void {
    this._commands$
      .pipe(
        concatMap((command) => {
          const handler = this._handlers.get(command.type);
          const prevState = this.currentState;

          if (!handler) {
            console.warn(
              `[RxCommandBus] No handler for command type: ${command.type}`
            );
            return of({ command, prevState, nextState: prevState });
          }

          try {
            const result = handler(prevState, command);
            return from(Promise.resolve(result)).pipe(
              catchError((error) => {
                console.error('[RxCommandBus] Handler error:', error);
                this._options.onCommandError?.({ command, error, state: prevState });
                return of(prevState);
              }),
              concatMap((nextState) => of({ command, prevState, nextState }))
            );
          } catch (error) {
            console.error('[RxCommandBus] Handler error:', error);
            this._options.onCommandError?.({ command, error, state: prevState });
            return of({ command, prevState, nextState: prevState });
          }
        }),
        catchError((error) => {
          console.error('[RxCommandBus] Processing error:', error);
          return of({
            command: { type: '__UNKNOWN__', payload: {} } as any,
            prevState: this.currentState,
            nextState: this.currentState,
          });
        })
      )
      .subscribe(({ command, prevState, nextState }) => {
        if (nextState !== this.currentState) {
          this._state.next(nextState);
        }
        this._options.onAfterCommand?.({ command, prevState, nextState });
      });
  }
}
