import { ManagedModelInfo } from "@/types";

export function findBestMatchingModel(
  models: ManagedModelInfo[],
  slug: string
): ManagedModelInfo | null {
  const normalizedSlug = String(slug || "").trim();
  if (!normalizedSlug) {
    return null;
  }

  const exact = models.find((item) => item.slug === normalizedSlug);
  if (exact) {
    return exact;
  }

  const lowerSlug = normalizedSlug.toLowerCase();
  return (
    models.find((item) => item.slug.toLowerCase() === lowerSlug) ?? null
  );
}
