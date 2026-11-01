export function capitalize(s: string) { return s.charAt(0).toUpperCase() + s.slice(1); }
export function slugify(s: string) { return s.toLowerCase().replace(/\s+/g, "-"); }
export default { capitalize, slugify };