/// <reference types="vite/client" />

declare module '@sohumsuthar/liquid-glass' {
  import type { CSSProperties, PropsWithChildren } from 'react'

  export function LiquidGlass(
    props: PropsWithChildren<{
      macro?: boolean
      mobileFlat?: boolean
      className?: string
      style?: CSSProperties
      contentClassName?: string
      contentStyle?: CSSProperties
    }>,
  ): React.JSX.Element

  export function LiquidGlassFilter(props: {
    displacementMap?: string
  }): React.JSX.Element
}
