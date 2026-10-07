import styles from './LoginModal.module.css';

/** 表单错误提示（原 renderError，全部认证视图共用） */
export default function ErrorHint({ message }: { message: string }) {
  return (
    <div className={styles.errorMessage} role="alert" data-testid="login-error-message">
      {message}
    </div>
  );
}
