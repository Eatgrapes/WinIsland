import { useState } from 'react'
import { Bell, Mail, Settings } from 'lucide-react'
import allByMyDesignCover from '../assets/all-by-my-design.jpg'
import type { DemoMode, Locale } from '../content'
import { copy } from '../content'

type ProductDemoProps = {
  locale: Locale
  mode?: DemoMode
  interactive?: boolean
  variant?: 'hero' | 'stage'
}

const PLAY_PATH = 'M187.2 100.9C174.8 94.1 159.8 94.4 147.6 101.6C135.4 108.8 128 121.9 128 136L128 504C128 518.1 135.5 531.2 147.6 538.4C159.7 545.6 174.8 545.9 187.2 539.1L523.2 355.1C536 348.1 544 334.6 544 320C544 305.4 536 291.9 523.2 284.9L187.2 100.9Z'
const PAUSE_PATH = 'M176 96C149.5 96 128 117.5 128 144L128 496C128 522.5 149.5 544 176 544L240 544C266.5 544 288 522.5 288 496L288 144C288 117.5 266.5 96 240 96L176 96ZM400 96C373.5 96 352 117.5 352 144L352 496C352 522.5 373.5 544 400 544L464 544C490.5 544 512 522.5 512 496L512 144C512 117.5 490.5 96 464 96L400 96Z'

function Spectrum({ playing }: { playing: boolean }) {
  return (
    <span className={playing ? 'spectrum is-playing' : 'spectrum'} aria-hidden="true">
      <i /><i /><i /><i /><i /><i />
    </span>
  )
}

function Cover({ compact = false }: { compact?: boolean }) {
  return <img className={compact ? 'album-art is-compact' : 'album-art'} src={allByMyDesignCover} alt="" />
}

function PlaybackGlyph({ playing }: { playing: boolean }) {
  return (
    <svg className="playback-glyph" viewBox="0 0 640 640" aria-hidden="true">
      <path d={playing ? PAUSE_PATH : PLAY_PATH} />
    </svg>
  )
}

function SkipControl({ direction }: { direction: 'previous' | 'next' }) {
  return (
    <span className={`skip-control skip-control--${direction}`} aria-hidden="true">
      <svg viewBox="0 0 640 640"><path d={PLAY_PATH} /></svg>
      <svg viewBox="0 0 640 640"><path d={PLAY_PATH} /></svg>
    </span>
  )
}

function NotificationView({ locale }: { locale: Locale }) {
  const text = copy[locale].demo
  return (
    <div className="notification-view">
      <span className="notification-icon"><Mail /></span>
      <div className="notification-copy">
        <small>{text.notificationApp}</small>
        <strong>{text.notification}</strong>
        <span>{text.notificationBody}</span>
      </div>
    </div>
  )
}

function CompactMusic({ lyric, playing }: { lyric: string; playing: boolean }) {
  return (
    <div className="compact-island" aria-hidden="true">
      <Cover compact />
      <span className="compact-lyric">{lyric}</span>
      <Spectrum playing={playing} />
    </div>
  )
}

export function ProductDemo({
  locale,
  mode = 'media',
  interactive = true,
  variant = 'stage',
}: ProductDemoProps) {
  const [playing, setPlaying] = useState(true)
  const text = copy[locale].demo

  if (variant === 'hero') {
    return (
      <div className="product-demo product-demo--hero">
        <span className="demo-halo" aria-hidden="true" />
        <div className="hero-morph" aria-hidden="true">
          <span className="morph-orbit" />
          <span className="morph-outline morph-outline--wide" />
          <span className="morph-outline morph-outline--compact" />
          <span className="morph-badge morph-badge--alert"><Bell /></span>
          <span className="morph-badge morph-badge--settings"><Settings /></span>
          <CompactMusic lyric={text.lyric} playing={playing} />
        </div>
      </div>
    )
  }

  return (
    <div className={`product-demo product-demo--stage product-demo--${mode}`}>
      <span className="demo-halo" aria-hidden="true" />
      <div className={`island island--${mode}`}>
        {mode === 'media' && (
          <div className="media-view">
            <Cover />
            <div className="island-copy"><strong>{text.track}</strong><small>{text.artist}</small></div>
            <Spectrum playing={playing} />
            <div className="progress-row" aria-hidden="true">
              <small>1:42</small><span><i /></span><small>−2:21</small>
            </div>
            <div className="playback-controls">
              <button type="button" aria-label="Previous track"><SkipControl direction="previous" /></button>
              <button
                type="button"
                className="playback-main"
                aria-label={playing ? 'Pause' : 'Play'}
                onClick={() => interactive && setPlaying((value) => !value)}
              >
                <PlaybackGlyph playing={playing} />
              </button>
              <button type="button" aria-label="Next track"><SkipControl direction="next" /></button>
            </div>
            <span className="page-edge page-edge--right" aria-hidden="true" />
          </div>
        )}

        {mode === 'notification' && <NotificationView locale={locale} />}

        {mode === 'widgets' && (
          <div className="widgets-view">
            <div className="native-widget native-widget--clock">09:41</div>
            <div className="native-widget native-widget--calendar">
              <small>{text.month}</small><strong>18</strong><span>{text.weekday}</span>
            </div>
            <div className="native-widget native-widget--settings"><Settings /></div>
            <span className="page-edge page-edge--left" aria-hidden="true" />
          </div>
        )}
      </div>
    </div>
  )
}
