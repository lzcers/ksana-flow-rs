/**
 * RxCommandBus - Command 总线
 * 使用 RxJS Subject 分发 Command，由注册的处理器处理
 */

import { Subject, Observable, of } from 'rxjs';
import { filter, map, mergeMap, catchError } from 'rxjs/operators';
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
        mergeMap((command) => {
          const handler = this._handlers.get(command.type);

          if (!handler) {
            console.warn(
              `[RxCommandBus] No handler for command type: ${command.type}`
            );
            return of(this.currentState);
          }

          try {
            const result = handler(this.currentState, command);

            // 处理异步结果
            if (result instanceof Promise) {
              return result.then(
                (newState) => newState,
                (error) => {
                  console.error('[RxCommandBus] Async handler error:', error);
                  return this.currentState;
                }
              );
            }

            return of(result);
          } catch (error) {
            console.error('[RxCommandBus] Handler error:', error);
            return of(this.currentState);
          }
        }),
        catchError((error) => {
          console.error('[RxCommandBus] Processing error:', error);
          return of(this.currentState);
        })
      )
      .subscribe((newState) => {
        if (newState !== this.currentState) {
          this._state.next(newState);
        }
      });
  }
}
