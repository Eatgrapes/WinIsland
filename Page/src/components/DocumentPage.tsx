import { useMemo } from 'react'
import { BookOpen, ChevronRight } from 'lucide-react'
import ReactMarkdown from 'react-markdown'
import { Link } from 'react-router-dom'
import remarkGfm from 'remark-gfm'
import { copy, DOC_KEYS, localePath, type DocKey, type Locale } from '../content'
import { docs } from '../docs'

export default function DocumentPage({ locale, page }: { locale: Locale; page: DocKey }) {
  const text = copy[locale]
  const markdown = docs[locale][page]

  const markdownComponents = useMemo(
    () => ({
      a: ({ href, children, ...props }: React.ComponentPropsWithoutRef<'a'>) => {
        if (!href || href.startsWith('http') || href.startsWith('mailto:')) {
          return (
            <a
              href={href}
              target={href?.startsWith('http') ? '_blank' : undefined}
              rel="noreferrer"
              {...props}
            >
              {children}
            </a>
          )
        }
        const normalized = href.startsWith('/zh/')
          ? href
          : localePath(locale, href.startsWith('/') ? href : `/${href}`)
        return <Link to={normalized}>{children}</Link>
      },
    }),
    [locale],
  )

  return (
    <div className="docs-page page-top section-shell">
      <aside className="docs-sidebar">
        <span>{text.docs.onThisPage}</span>
        <nav aria-label={text.docs.onThisPage}>
          {DOC_KEYS.map((key) => (
            <Link
              key={key}
              className={page === key ? 'is-active' : ''}
              to={localePath(locale, `/${key}`)}
            >
              {text.docs.pages[key]}
            </Link>
          ))}
        </nav>
      </aside>
      <article className="markdown-body">
        <div className="docs-breadcrumb">
          <BookOpen size={16} />
          {text.docs.title}
          <ChevronRight size={14} />
          {text.docs.pages[page]}
        </div>
        <ReactMarkdown remarkPlugins={[remarkGfm]} components={markdownComponents}>
          {markdown}
        </ReactMarkdown>
      </article>
    </div>
  )
}
