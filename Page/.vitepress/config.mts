import { defineConfig } from 'vitepress'

export default defineConfig({
  base: '/WinIsland/',
  title: "WinIsland",
  description: "A sleek, functional dynamic island for Windows",
  head: [
    ['link', { rel: 'icon', href: '/WinIsland/icon.png' }]
  ],
  locales: {
    root: {
      label: 'English',
      lang: 'en',
      themeConfig: {
        nav: [
          { text: 'Home', link: '/' },
          { text: 'Guide', link: '/guide' },
          { text: 'Changelog', link: '/changelog' },
          { text: 'Download', link: '/download' }
        ],
        sidebar: [
          {
            text: 'Guide',
            items: [
              { text: 'What is WinIsland?', link: '/guide' },
              { text: 'Getting Started', link: '/getting-started' }
            ]
          },
          {
            text: 'Changelog',
            items: [
              { text: 'Changelog', link: '/changelog' }
            ]
          },
          {
            text: 'Download',
            items: [
              { text: 'Latest Release', link: '/download#official-releases' },
              { text: 'Latest Nightly', link: '/download#nightly-builds' }
            ]
          },
          {
            text: 'Developer',
            items: [
              { text: 'Plugin Development', link: '/plugin-dev' },
              { text: 'API Changelog', link: '/api-changelog' }
            ]
          }
        ],
        footer: {
          message: 'Released under the GNU GPL v3 License.',
          copyright: 'Copyright © 2026-present WinIsland'
        }
      }
    },
    zh: {
      label: '简体中文',
      lang: 'zh',
      link: '/zh/',
      themeConfig: {
        nav: [
          { text: '首页', link: '/zh/' },
          { text: '指南', link: '/zh/guide' },
          { text: '更新日志', link: '/zh/changelog' },
          { text: '下载', link: '/zh/download' }
        ],
        sidebar: [
          {
            text: '指南',
            items: [
              { text: '什么是 WinIsland？', link: '/zh/guide' },
              { text: '快速开始', link: '/zh/getting-started' }
            ]
          },
          {
            text: '更新日志',
            items: [
              { text: '更新日志', link: '/zh/changelog' }
            ]
          },
          {
            text: '下载',
            items: [
              { text: '最新正式版', link: '/zh/download#官方正式版-release' },
              { text: '最新预览版', link: '/zh/download#预览版-nightly' }
            ]
          },
          {
            text: '开发者',
            items: [
              { text: '插件开发', link: '/zh/plugin-dev' },
              { text: 'API 更新日志', link: '/zh/api-changelog' }
            ]
          }
        ],
        footer: {
          message: '基于 GNU GPL v3 许可发布。',
          copyright: '版权所有 © 2026-present WinIsland'
        },
        docFooter: {
          prev: '上一页',
          next: '下一页'
        },
        outline: {
          label: '页面导航'
        },
        returnToTopLabel: '回到顶部',
        sidebarMenuLabel: '菜单',
        darkModeSwitchLabel: '主题',
        lightModeSwitchTitle: '切换到浅色模式',
        darkModeSwitchTitle: '切换到深色模式'
      }
    }
  },
  themeConfig: {
    socialLinks: [
      { icon: 'github', link: 'https://github.com/Eatgrapes/WinIsland' }
    ]
  }
})
