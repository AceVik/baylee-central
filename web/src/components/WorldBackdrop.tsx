// The approved world is shared by every route; it never receives input.
export function WorldBackdrop({ companion }: { companion: boolean }) {
  return (
    <div className="world-backdrop" aria-hidden="true">
      <div className="world-landscape" />
      <div className="world-wash" />
      {companion && <img className="world-companion" src="/img/baylee-cloud.webp" width={1200} height={800} alt="" decoding="async" />}
    </div>
  );
}
