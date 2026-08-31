import { t } from "i18next";
/**
 * 图标优化工具
 * 用于优化和统一项目中的图标系统
 */

/**
 * 图标配置接口
 */
interface IconConfig {
  size?: number;
  color?: string;
  strokeWidth?: number;
  className?: string;
}

/**
 * 图标数据接口
 */
interface IconData {
  name: string;
  svg: string;
  category: string;
  description: string;
}

/**
 * 图标优化器类
 */
class IconOptimizer {
  private defaultConfig: Required<IconConfig> = {
    size: 24,
    color: 'var(--nt-green)',
    strokeWidth: 1.5,
    className: 'nt-icon'
  };

  /**
   * 优化SVG图标
   */
  optimizeSVG(svgContent: string, config: IconConfig = {}): string {
    const mergedConfig = {
      ...this.defaultConfig,
      ...config
    };

    // 解析SVG内容
    const parser = new DOMParser();
    const svgDoc = parser.parseFromString(svgContent, 'image/svg+xml');
    const svgElement = svgDoc.documentElement;

    // 设置属性
    svgElement.setAttribute('width', mergedConfig.size.toString());
    svgElement.setAttribute('height', mergedConfig.size.toString());
    svgElement.setAttribute('fill', 'none');
    svgElement.setAttribute('stroke', mergedConfig.color);
    svgElement.setAttribute('stroke-width', mergedConfig.strokeWidth.toString());
    svgElement.setAttribute('stroke-linecap', 'square');
    svgElement.setAttribute('stroke-linejoin', 'miter');

    // 添加类名
    if (mergedConfig.className) {
      svgElement.setAttribute('class', mergedConfig.className);
    }

    // 序列化回字符串
    return new XMLSerializer().serializeToString(svgElement);
  }

  /**
   * 创建图标组件
   */
  createIconComponent(iconData: IconData, config: IconConfig = {}): string {
    const optimizedSVG = this.optimizeSVG(iconData.svg, config);
    return `
import React from 'react'

interface ${iconData.name}IconProps {
  size?: number
  color?: string
  className?: string
}

export const ${iconData.name}Icon: React.FC<${iconData.name}IconProps> = ({
  size = 24,
  color = '#00FF00',
  className = ''
}) => {
  return (
    <div
      className={\`nt-icon \${className}\`}
      // T2.12 XSS 审查：此处 SVG 来自代码生成时的静态优化字符串，非用户输入，理论安全
      dangerouslySetInnerHTML={{
        __html: \`${optimizedSVG.replace(/`/g, '\\`')}\`
      }}
      style={{
        width: size,
        height: size,
        display: 'inline-flex',
        alignItems: 'center',
        justifyContent: 'center'
      }}
    />
  )
}
    `.trim();
  }

  /**
   * 批量优化图标
   */
  batchOptimizeIcons(icons: IconData[], config: IconConfig = {}): Record<string, string> {
    const result: Record<string, string> = {};
    icons.forEach(icon => {
      result[icon.name] = this.createIconComponent(icon, config);
    });
    return result;
  }

  /**
   * 生成图标索引文件
   */
  generateIconIndex(icons: IconData[]): string {
    const imports = icons.map(icon => `export { ${icon.name}Icon } from './${icon.name}Icon'`).join('\n');
    return t("utils.iconOptimizer.k1", {
      imports: imports,
      arg0: icons.map(icon => `'${icon.name}'`).join(' | '),
      arg1: icons.map(icon => `'${icon.name}': ${icon.name}Icon`).join(',\n  ')
    }).trim();
  }
}

/**
 * 图标管理器
 */
export class IconManager {
  private static instance: IconManager;
  private icons: Map<string, IconData> = new Map();
  private optimizer = new IconOptimizer();
  private constructor() {}
  static getInstance(): IconManager {
    if (!IconManager.instance) {
      IconManager.instance = new IconManager();
    }
    return IconManager.instance;
  }

  /**
   * 注册图标
   */
  registerIcon(iconData: IconData): void {
    this.icons.set(iconData.name, iconData);
  }

  /**
   * 获取图标
   */
  getIcon(name: string): IconData | undefined {
    return this.icons.get(name);
  }

  /**
   * 获取所有图标
   */
  getAllIcons(): IconData[] {
    return Array.from(this.icons.values());
  }

  /**
   * 生成图标组件
   */
  generateIconComponent(name: string, config: IconConfig = {}): string | null {
    const iconData = this.getIcon(name);
    if (!iconData) return null;
    return this.optimizer.createIconComponent(iconData, config);
  }

  /**
   * 生成所有图标组件
   */
  generateAllIconComponents(config: IconConfig = {}): Record<string, string> {
    return this.optimizer.batchOptimizeIcons(this.getAllIcons(), config);
  }

  /**
   * 生成图标索引
   */
  generateIconIndex(): string {
    return this.optimizer.generateIconIndex(this.getAllIcons());
  }
}

/**
 * 预定义图标数据
 */
const predefinedIcons: IconData[] = [{
  name: 'Home',
  svg: '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path d="M3 9l9-7 9 7v11a2 2 0 01-2 2H5a2 2 0 01-2-2z"/><polyline points="9 22 9 12 15 12 15 22"/></svg>',
  category: 'navigation',
  description: t("utils.iconOptimizer.k2")
}, {
  name: 'Profile',
  svg: '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path d="M20 21v-2a4 4 0 00-4-4H8a4 4 0 00-4 4v2"/><circle cx="12" cy="7" r="4"/></svg>',
  category: 'user',
  description: t("utils.iconOptimizer.k3")
}, {
  name: 'Settings',
  svg: '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.65 1.65 0 00.33 1.82l.06.06a2 2 0 010 2.83 2 2 0 01-2.83 0l-.06-.06a1.65 1.65 0 00-1.82-.33 1.65 1.65 0 00-1 1.51V21a2 2 0 01-2 2 2 2 0 01-2-2v-.09A1.65 1.65 0 009 19.4a1.65 1.65 0 00-1.82.33l-.06.06a2 2 0 01-2.83 0 2 2 0 010-2.83l.06-.06a1.65 1.65 0 00.33-1.82 1.65 1.65 0 00-1.51-1H3a2 2 0 01-2-2 2 2 0 012-2h.09A1.65 1.65 0 004.6 9a1.65 1.65 0 00-.33-1.82l-.06-.06a2 2 0 010-2.83 2 2 0 012.83 0l.06.06a1.65 1.65 0 001.82.33H9a1.65 1.65 0 001-1.51V3a2 2 0 012-2 2 2 0 012 2v.09a1.65 1.65 0 001 1.51 1.65 1.65 0 001.82-.33l.06-.06a2 2 0 012.83 0 2 2 0 010 2.83l-.06.06a1.65 1.65 0 00-.33 1.82V9a1.65 1.65 0 001.51 1H21a2 2 0 012 2 2 2 0 01-2 2h-.09a1.65 1.65 0 00-1.51 1z"/></svg>',
  category: 'system',
  description: t("utils.iconOptimizer.k4")
}, {
  name: 'Trash',
  svg: '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><polyline points="3 6 5 6 21 6"/><path d="M19 6v14a2 2 0 01-2 2H7a2 2 0 01-2-2V6m3 0V4a2 2 0 012-2h4a2 2 0 012 2v2"/></svg>',
  category: 'action',
  description: t("utils.iconOptimizer.k5")
}];

// 初始化图标管理器并注册预定义图标
const iconManager = IconManager.getInstance();
predefinedIcons.forEach(icon => iconManager.registerIcon(icon));

/**
 * 导出工具函数
 */
export { iconManager };
export default IconOptimizer;