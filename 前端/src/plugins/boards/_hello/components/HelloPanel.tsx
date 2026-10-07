/** 演示插槽组件：插入 _hello 声明的 hello.panel 插槽（SlotRenderer 渲染） */
export default function HelloPanel() {
  return (
    <div
      style={{
        padding: 12,
        border: '1px solid #2a3f3a',
        borderRadius: 8,
        background: 'rgba(0, 255, 157, 0.04)',
      }}
      data-hello-panel
    >
      <div style={{ fontSize: 12, color: '#00ff9d', marginBottom: 4 }}>hello.panel 插槽组件</div>
      <div style={{ fontSize: 13, color: '#c0c0c0' }}>
        该卡片由 _hello 插件经 SlotRenderer 挂载（阶段1 演示）
      </div>
    </div>
  );
}
