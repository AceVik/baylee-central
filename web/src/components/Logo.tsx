// The logo (our own, Baylee's docs/legal.md §10) at the sizes it is shown:
// AVIF where the browser takes it, WebP otherwise.

export function Logo({
  sizes,
  alt,
  className,
  lazy,
  priority,
}: {
  sizes: string;
  alt: string;
  className?: string;
  lazy?: boolean;
  priority?: boolean;
}) {
  return (
    <picture>
      <source type="image/avif" srcSet="/img/baylee-logo-400.avif 400w, /img/baylee-logo-800.avif 800w" sizes={sizes} />
      <source type="image/webp" srcSet="/img/baylee-logo-400.webp 400w, /img/baylee-logo.webp 800w" sizes={sizes} />
      <img
        className={className}
        src="/img/baylee-logo.webp"
        width={800}
        height={336}
        alt={alt}
        decoding="async"
        loading={lazy === true ? "lazy" : "eager"}
        fetchPriority={priority === true ? "high" : "auto"}
      />
    </picture>
  );
}
