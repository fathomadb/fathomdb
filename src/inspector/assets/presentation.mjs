/** Return only exact bounded node declarations that actually declare property FTS. */
export function propertyChoices(catalog) {
  if(!catalog || catalog.evicted)return [];
  const length=value=>new TextEncoder().encode(value).length.toString();
  return (catalog.projections??[]).slice(0,200).filter(p=>(p.entity_class===undefined||p.entity_class==='node')&&typeof p.name==='string'&&p.name_complete!==false&&length(p.name)===p.name_bytes&&typeof p.roles==='string'&&length(p.roles)===p.roles_bytes&&p.roles.split(',').map(v=>v.trim()).includes('searchable')&&typeof p.recorded_tokenizer==='string'&&length(p.recorded_tokenizer)===p.tokenizer_bytes).map(p=>p.name);
}

/** Render exact integer epoch units only within the ECMAScript Date range. */
export function epochUTC(value,unit='seconds') {
  if(typeof value!=='string'||!/^[-]?\d+$/.test(value)||!['seconds','milliseconds'].includes(unit))return null;
  const ms=BigInt(value)*(unit==='seconds'?1000n:1n);
  if(ms>8640000000000000n||ms< -8640000000000000n)return null;
  return new Date(Number(ms)).toISOString();
}
