NOT EXISTS (
 SELECT 1 FROM _fathomdb_artifact_revisions ar
 LEFT JOIN _fathomdb_source_links link ON link.artifact_revision_id=ar.revision_id
 WHERE ar.write_cursor=f.revision AND ar.artifact_role='derived_semantic'
 AND (
   link.artifact_revision_id IS NULL OR NOT EXISTS (
    SELECT 1 FROM _fathomdb_artifact_revisions source
    JOIN canonical_nodes source_node ON source_node.write_cursor=source.write_cursor
    WHERE source.revision_id=link.source_revision_id AND source.artifact_role='canonical_source'
    AND source.completeness='complete' AND source_node.state='active' AND source_node.superseded_at IS NULL
    AND (source_node.valid_from IS NULL OR source_node.valid_from<=?4)
    AND (source_node.valid_until IS NULL OR source_node.valid_until>?4)
   ) OR EXISTS (
    SELECT 1 FROM _fathomdb_dependency_closures closure
    WHERE closure.phase!='complete' AND (
      (closure.root_kind='source_revision' AND closure.root_value=link.source_revision_id) OR
      (closure.root_kind='source_bucket' AND closure.root_value=link.source_id)
    )
   )
 )
)

AND NOT EXISTS(SELECT 1 FROM _fathomdb_dependency_closures bucket WHERE bucket.root_kind='source_bucket' AND bucket.root_value=f.source AND bucket.phase!='complete')
