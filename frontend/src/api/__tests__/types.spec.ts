import { describe, expect, it } from 'vitest';

// ── Inline type stubs ──
// These mirror types that will be added to @/api/types in Phase 6 (Task 6.1).
// Remove these inline stubs and import from @/api/types once Task 6.1 is complete.

interface MultiQueryData {
  original_query: string;
  variants: string[];
  latency_ms: number;
}

interface HydeResultData {
  query: string;
  hypothetical_doc: string;
  latency_ms: number;
}

interface HydeData {
  per_query: HydeResultData[];
}

interface KeywordSearchData {
  query_tokens: string[];
  total_matches: number;
  results: {
    chunk_id: string;
    document_name: string;
    chunk_index: number;
    score: number;
    text_snippet: string;
  }[];
  latency_ms: number;
}

interface MergeSourceBreakdown {
  vector_chunks: number;
  keyword_chunks: number;
}

interface MergeDedupData {
  input_chunks: number;
  after_dedup: number;
  source_breakdown: MergeSourceBreakdown;
}

interface RerankResultData {
  chunk_id: string;
  score: number;
  verdict: string;
  comment: string;
}

interface RerankingData {
  input_count: number;
  accepted: number;
  rejected: number;
  results: RerankResultData[];
}

interface PipelineStageEvent {
  stage: string;
  data: Record<string, unknown>;
  latency_ms: number;
}

interface ExtendedSourceRef {
  document_id: string;
  document_name: string;
  chunk_index: number;
  text: string;
  relevance: number;
  stage?: string;
  rerank_score?: number;
  rerank_verdict?: string;
  rerank_comment?: string;
  keyword_matches?: string[];
}

interface PipelineDebugData {
  query_text: string;
  multi_query: MultiQueryData | null;
  hyde: HydeData | null;
  embedding_search: {
    query_snippet: string;
    embedding_dimension: number;
    latency_ms: number;
    collection_name: string;
    top_k: number;
    result_count: number;
    retries: number;
    results: {
      chunk_id: string;
      document_name: string;
      chunk_index: number;
      score: number;
      text_snippet: string;
    }[];
  } | null;
  keyword_search: KeywordSearchData | null;
  merge_dedup: MergeDedupData | null;
  reranking: RerankingData | null;
  final_answer: {
    model: string;
    max_retries: number;
    chunks_in_context: number;
    history_message_count: number;
    history_token_estimate: number;
    token_budget: number;
    total_tokens_estimate: number;
    latency_ms: number;
    prompt_preview: string;
  } | null;
}

// ── Tests ──

describe('PipelineStageEvent shape', () => {
  it('mock pipeline stage JSON produces correct typed object', () => {
    const json = JSON.parse(
      '{"stage":"expanded_questions","data":{"original_query":"test","variants":["q1","q2"],"latency_ms":100},"latency_ms":100}',
    );
    const event = json as PipelineStageEvent;

    expect(event.stage).toBe('expanded_questions');
    expect(typeof event.latency_ms).toBe('number');
    expect(event.latency_ms).toBe(100);
    expect(typeof event.data).toBe('object');
    expect(event.data).not.toBeNull();

    // When data holds a MultiQueryData, its shape should be correct
    const mq = event.data as MultiQueryData;
    expect(mq.original_query).toBe('test');
    expect(Array.isArray(mq.variants)).toBe(true);
    expect(mq.variants).toHaveLength(2);
    expect(mq.latency_ms).toBe(100);
  });

  it('handles hyde_docs stage shape', () => {
    const event: PipelineStageEvent = {
      stage: 'hyde_docs',
      data: {
        per_query: [
          {
            query: 'How to configure rate limiting?',
            hypothetical_doc: 'Rate limiting is configured via backend middleware...',
            latency_ms: 800,
          },
        ],
      },
      latency_ms: 850,
    };

    expect(event.stage).toBe('hyde_docs');
    expect(Array.isArray((event.data as HydeData).per_query)).toBe(true);
    expect((event.data as HydeData).per_query).toHaveLength(1);
    expect((event.data as HydeData).per_query[0].query).toContain('rate limiting');
    expect(typeof (event.data as HydeData).per_query[0].latency_ms).toBe('number');
  });

  it('handles keyword_matches stage shape', () => {
    const event: PipelineStageEvent = {
      stage: 'keyword_matches',
      data: {
        query_tokens: ['rate', 'limiting'],
        total_matches: 5,
        latency_ms: 10,
        results: [
          {
            chunk_id: 'chunk-1',
            document_name: 'config.md',
            chunk_index: 0,
            score: 2.5,
            text_snippet: 'Rate limiting is...',
          },
        ],
      },
      latency_ms: 12,
    };

    expect(event.stage).toBe('keyword_matches');
    const kw = event.data as KeywordSearchData;
    expect(kw.query_tokens).toEqual(['rate', 'limiting']);
    expect(kw.total_matches).toBe(5);
    expect(kw.results).toHaveLength(1);
    expect(kw.results[0].chunk_id).toBe('chunk-1');
  });

  it('handles merged_chunks stage shape', () => {
    const event: PipelineStageEvent = {
      stage: 'merged_chunks',
      data: {
        input_chunks: 12,
        after_dedup: 8,
        source_breakdown: {
          vector_chunks: 5,
          keyword_chunks: 3,
        },
      },
      latency_ms: 2,
    };

    expect(event.stage).toBe('merged_chunks');
    const md = event.data as MergeDedupData;
    expect(md.input_chunks).toBe(12);
    expect(md.after_dedup).toBe(8);
    expect(md.source_breakdown.vector_chunks).toBe(5);
    expect(md.source_breakdown.keyword_chunks).toBe(3);
  });

  it('handles reranked_chunks stage shape', () => {
    const event: PipelineStageEvent = {
      stage: 'reranked_chunks',
      data: {
        input_count: 8,
        accepted: 5,
        rejected: 3,
        results: [
          {
            chunk_id: 'chunk-1',
            score: 9,
            verdict: 'брать',
            comment: 'Directly answers the question',
          },
        ],
      },
      latency_ms: 2500,
    };

    expect(event.stage).toBe('reranked_chunks');
    const rr = event.data as RerankingData;
    expect(rr.input_count).toBe(8);
    expect(rr.accepted).toBe(5);
    expect(rr.rejected).toBe(3);
    expect(rr.results[0].verdict).toBe('брать');
    expect(rr.results[0].score).toBe(9);
  });
});

describe('Extended SourceRef shape', () => {
  it('SourceRef with stage/rerank/verdict fields works', () => {
    const ref: ExtendedSourceRef = {
      document_id: 'doc-1',
      document_name: 'test.md',
      chunk_index: 0,
      text: 'some text',
      relevance: 0.95,
      stage: 'reranked',
      rerank_score: 8,
      rerank_verdict: 'брать',
      rerank_comment: 'Relevant to query',
      keyword_matches: ['test', 'config'],
    };

    expect(ref.document_id).toBe('doc-1');
    expect(ref.stage).toBe('reranked');
    expect(ref.rerank_score).toBe(8);
    expect(ref.rerank_verdict).toBe('брать');
    expect(ref.rerank_comment).toBe('Relevant to query');
    expect(ref.keyword_matches).toEqual(['test', 'config']);
  });

  it('optional fields are undefined by default for backward compat', () => {
    // Simulate a SourceRef without the new optional fields (legacy format)
    const ref: ExtendedSourceRef = {
      document_id: 'doc-1',
      document_name: 'test.md',
      chunk_index: 0,
      text: 'some text',
      relevance: 0.95,
    };

    expect(ref.document_id).toBe('doc-1');
    expect(ref.stage).toBeUndefined();
    expect(ref.rerank_score).toBeUndefined();
    expect(ref.rerank_verdict).toBeUndefined();
    expect(ref.rerank_comment).toBeUndefined();
    expect(ref.keyword_matches).toBeUndefined();
  });
});

describe('DebugData with all pipeline steps', () => {
  it('DebugData with populated multi_query/hyde/etc. produces correct shape', () => {
    const debugData: PipelineDebugData = {
      query_text: 'How to configure rate limiting?',
      multi_query: {
        original_query: 'How to configure rate limiting?',
        variants: ['How do I set up rate limiting?', 'Rate limiting configuration guide'],
        latency_ms: 1200,
      },
      hyde: {
        per_query: [
          {
            query: 'How do I set up rate limiting?',
            hypothetical_doc:
              'Rate limiting is configured in the backend middleware. The system uses token bucket algorithm...',
            latency_ms: 800,
          },
        ],
      },
      embedding_search: {
        query_snippet: 'rate limiting',
        embedding_dimension: 384,
        latency_ms: 150,
        collection_name: 'default',
        top_k: 5,
        result_count: 3,
        retries: 0,
        results: [
          {
            chunk_id: 'chunk-1',
            document_name: 'config.md',
            chunk_index: 0,
            score: 0.92,
            text_snippet: 'Rate limiting is configured via...',
          },
        ],
      },
      keyword_search: {
        query_tokens: ['rate', 'limiting', 'configure'],
        total_matches: 10,
        latency_ms: 5,
        results: [
          {
            chunk_id: 'chunk-kw-1',
            document_name: 'config.md',
            chunk_index: 0,
            score: 3.2,
            text_snippet: 'Rate limiting is...',
          },
        ],
      },
      merge_dedup: {
        input_chunks: 12,
        after_dedup: 8,
        source_breakdown: {
          vector_chunks: 5,
          keyword_chunks: 3,
        },
      },
      reranking: {
        input_count: 8,
        accepted: 5,
        rejected: 3,
        results: [
          {
            chunk_id: 'chunk-1',
            score: 9,
            verdict: 'брать',
            comment: 'Directly answers the question',
          },
          {
            chunk_id: 'chunk-kw-1',
            score: 4,
            verdict: 'не брать',
            comment: 'Only partially relevant',
          },
        ],
      },
      final_answer: {
        model: 'gpt-4o',
        max_retries: 3,
        chunks_in_context: 5,
        history_message_count: 2,
        history_token_estimate: 500,
        token_budget: 4000,
        total_tokens_estimate: 2500,
        latency_ms: 3500,
        prompt_preview: 'You are a helpful assistant...',
      },
    };

    // Query text
    expect(debugData.query_text).toBe('How to configure rate limiting?');

    // Multi-query step
    expect(debugData.multi_query).not.toBeNull();
    expect(debugData.multi_query?.original_query).toBe('How to configure rate limiting?');
    expect(debugData.multi_query?.variants).toHaveLength(2);
    expect(typeof debugData.multi_query?.latency_ms).toBe('number');

    // HyDE step
    expect(debugData.hyde).not.toBeNull();
    expect(debugData.hyde?.per_query).toHaveLength(1);
    expect(debugData.hyde?.per_query[0].query).toContain('rate limiting');
    expect(debugData.hyde?.per_query[0].hypothetical_doc).toContain('Rate limiting');

    // Embedding search (existing type)
    expect(debugData.embedding_search).not.toBeNull();
    expect(debugData.embedding_search?.collection_name).toBe('default');
    expect(debugData.embedding_search?.result_count).toBe(3);

    // Keyword search (new concrete type)
    expect(debugData.keyword_search).not.toBeNull();
    expect(debugData.keyword_search?.query_tokens).toContain('rate');
    expect(debugData.keyword_search?.total_matches).toBe(10);
    expect(debugData.keyword_search?.latency_ms).toBe(5);

    // Merge + dedup
    expect(debugData.merge_dedup).not.toBeNull();
    expect(debugData.merge_dedup?.input_chunks).toBe(12);
    expect(debugData.merge_dedup?.after_dedup).toBe(8);
    expect(debugData.merge_dedup?.source_breakdown.vector_chunks).toBe(5);
    expect(debugData.merge_dedup?.source_breakdown.keyword_chunks).toBe(3);

    // Reranking
    expect(debugData.reranking).not.toBeNull();
    expect(debugData.reranking?.input_count).toBe(8);
    expect(debugData.reranking?.accepted).toBe(5);
    expect(debugData.reranking?.rejected).toBe(3);
    expect(debugData.reranking?.results).toHaveLength(2);
    expect(debugData.reranking?.results[0].verdict).toBe('брать');
    expect(debugData.reranking?.results[1].verdict).toBe('не брать');

    // Final answer (existing type)
    expect(debugData.final_answer).not.toBeNull();
    expect(debugData.final_answer?.model).toBe('gpt-4o');
    expect(debugData.final_answer?.chunks_in_context).toBe(5);
  });

  it('all optional step fields can be null for partial debug data', () => {
    // This simulates what happens when only embedding_search completed
    const partial: PipelineDebugData = {
      query_text: 'test',
      multi_query: null,
      hyde: null,
      embedding_search: null,
      keyword_search: null,
      merge_dedup: null,
      reranking: null,
      final_answer: null,
    };

    expect(partial.query_text).toBe('test');
    expect(partial.multi_query).toBeNull();
    expect(partial.hyde).toBeNull();
    expect(partial.embedding_search).toBeNull();
    expect(partial.keyword_search).toBeNull();
    expect(partial.merge_dedup).toBeNull();
    expect(partial.reranking).toBeNull();
    expect(partial.final_answer).toBeNull();
  });
});

describe('Stage names enumeration', () => {
  it('recognizes all 6 pipeline stage types', () => {
    const stages = [
      'expanded_questions',
      'hyde_docs',
      'keyword_matches',
      'merged_chunks',
      'reranked_chunks',
      'pipeline_metric',
    ] as const;

    expect(stages).toHaveLength(6);
    expect(stages).toContain('expanded_questions');
    expect(stages).toContain('hyde_docs');
    expect(stages).toContain('keyword_matches');
    expect(stages).toContain('merged_chunks');
    expect(stages).toContain('reranked_chunks');
    expect(stages).toContain('pipeline_metric');
  });
});
