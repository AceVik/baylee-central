// The logo (our own, Baylee's docs/legal.md §10) at the sizes it is shown:
// The approved sunlight wordmark, in two WebP sizes.

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
      <source type="image/webp" srcSet="/img/baylee-sunlight-480.webp 480w, /img/baylee-sunlight-960.webp 960w" sizes={sizes} />
      <img
        className={className}
        src="/img/baylee-sunlight-960.webp"
        width={960}
        height={400}
        alt={alt}
        decoding="async"
        loading={lazy === true ? "lazy" : "eager"}
        fetchPriority={priority === true ? "high" : "auto"}
      />
    </picture>
  );
}
