import apiEn from '../api-changelog.md?raw'
import downloadEn from '../download.md?raw'
import gettingStartedEn from '../getting-started.md?raw'
import guideEn from '../guide.md?raw'
import pluginEn from '../plugin-dev.md?raw'
import apiZh from '../zh/api-changelog.md?raw'
import downloadZh from '../zh/download.md?raw'
import gettingStartedZh from '../zh/getting-started.md?raw'
import guideZh from '../zh/guide.md?raw'
import pluginZh from '../zh/plugin-dev.md?raw'
import changelogEn from '../../Changelog.md?raw'
import changelogZh from '../../Changelog-zh.md?raw'

export const docs = {
  en: {
    guide: guideEn,
    'getting-started': gettingStartedEn,
    download: downloadEn,
    'plugin-dev': pluginEn,
    'api-changelog': apiEn,
    changelog: changelogEn,
  },
  zh: {
    guide: guideZh,
    'getting-started': gettingStartedZh,
    download: downloadZh,
    'plugin-dev': pluginZh,
    'api-changelog': apiZh,
    changelog: changelogZh,
  },
} as const
