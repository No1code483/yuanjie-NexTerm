import { t } from "i18next";
import { Component, ErrorInfo, ReactNode } from 'react';
import styles from './ErrorBoundary.module.css';
interface Props {
  children: ReactNode;
  fallback?: ReactNode;
  /** 错误回调，用于上报 */
  onError?: (error: Error, errorInfo: ErrorInfo) => void;
  /** 组件名称，用于标识 */
  name?: string;
}
interface State {
  hasError: boolean;
  error: Error | null;
  errorInfo: ErrorInfo | null;
  resetKey: number;
}
export default class ErrorBoundary extends Component<Props, State> {
  public state: State = {
    hasError: false,
    error: null,
    errorInfo: null,
    resetKey: 0
  };
  public static getDerivedStateFromError(error: Error): Partial<State> {
    return {
      hasError: true,
      error
    };
  }
  public componentDidCatch(error: Error, errorInfo: ErrorInfo) {
    this.setState({
      errorInfo
    });
    console.error(`%c[ErrorBoundary${this.props.name ? ` · ${this.props.name}` : ''}] %c崩溃`, 'color: #FF0000; font-weight: 600', 'color: #FF0000', '\n', error, '\n', errorInfo.componentStack);
    this.props.onError?.(error, errorInfo);
  }
  private handleRetry = () => {
    this.setState(prev => ({
      hasError: false,
      error: null,
      errorInfo: null,
      resetKey: prev.resetKey + 1
    }));
  };
  private handleReload = () => {
    window.location.reload();
  };
  public render() {
    if (this.state.hasError) {
      if (this.props.fallback) {
        return <div key={this.state.resetKey}>
            {this.props.fallback}
          </div>;
      }
      return <div key={this.state.resetKey} className={styles.boundary}>
          <div className={styles.glitch} aria-hidden="true">ERROR</div>
          <div className={styles.content}>
            <div className={styles.icon}>[!]</div>
            <h2 className={styles.title}>
              {this.props.name ? t("components.ErrorBoundary.k1", {
              name: this.props.name
            }) : t("components.ErrorBoundary.k2")}
            </h2>
            <p className={styles.message}>
              {this.state.error?.message || t("components.ErrorBoundary.k3")}
            </p>
            {this.state.errorInfo && <details className={styles.details}>
                <summary className={styles.summary}>{t("components.ErrorBoundary.k4")}</summary>
                <pre className={styles.stack}>
                  {this.state.error?.stack}
                  {'\n\n--- Component Stack ---'}
                  {this.state.errorInfo.componentStack}
                </pre>
              </details>}
            <div className={styles.actions}>
              <button className={styles.btn} onClick={this.handleRetry}>
                {t("components.ErrorBoundary.k5")}
              </button>
              <button className={styles.btnReload} onClick={this.handleReload}>
                {t("common.refresh")}
              </button>
            </div>
          </div>
        </div>;
    }
    return <>{this.props.children}</>;
  }
}