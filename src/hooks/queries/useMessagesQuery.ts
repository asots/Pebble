import { useInfiniteQuery, type InfiniteData, type QueryClient } from "@tanstack/react-query";
import { listMessages } from "@/lib/api";
import type { MessageSummary } from "@/lib/api";

export const MESSAGES_PAGE_SIZE = 50;

export const messagesQueryKey = (folderId: string, folderIds?: string[]) =>
  ["messages", folderId, folderIds] as const;

/**
 * Paginated message list backed by React Query's useInfiniteQuery.
 * Each page is a MessageSummary[] fetched by offset; `data` flattens them
 * so render code keeps the flat-array shape it already expects.
 */
export function useMessagesQuery(
  folderId: string | null,
  folderIds?: string[],
) {
  const query = useInfiniteQuery({
    queryKey: messagesQueryKey(folderId ?? "", folderIds),
    queryFn: ({ pageParam }) =>
      listMessages(folderId!, MESSAGES_PAGE_SIZE, pageParam as number, folderIds),
    initialPageParam: 0,
    getNextPageParam: (lastPage, allPages) =>
      lastPage.length < MESSAGES_PAGE_SIZE ? undefined : allPages.length * MESSAGES_PAGE_SIZE,
    enabled: !!folderId,
    staleTime: 60_000,
  });

  const data: MessageSummary[] = query.data?.pages.flat() ?? [];
  return {
    data,
    isLoading: query.isLoading,
    hasNextPage: query.hasNextPage,
    isFetchingNextPage: query.isFetchingNextPage,
    fetchNextPage: query.fetchNextPage,
  };
}

export type MessagesInfiniteData = InfiniteData<MessageSummary[], number>;

/**
 * Patch every cached ["messages", ...] infinite-query entry with a transform
 * applied to each page's flat array. Used by optimistic flag/delete/move
 * updates so they don't have to know about the page shape.
 */
export function patchMessagesCache(
  queryClient: QueryClient,
  transform: (messages: MessageSummary[]) => MessageSummary[],
) {
  queryClient.setQueriesData<MessagesInfiniteData>(
    { queryKey: ["messages"] },
    (old) => {
      if (!old) return old;
      return { ...old, pages: old.pages.map(transform) };
    },
  );
}

/** Find the first cached message matching `predicate` across all pages. */
export function findCachedMessage(
  queryClient: QueryClient,
  predicate: (m: MessageSummary) => boolean,
): MessageSummary | undefined {
  const entries = queryClient.getQueriesData<MessagesInfiniteData>({ queryKey: ["messages"] });
  for (const [, data] of entries) {
    if (!data) continue;
    for (const page of data.pages) {
      const hit = page.find(predicate);
      if (hit) return hit;
    }
  }
  return undefined;
}

/** Read a flat snapshot of the first non-empty cached message list. */
export function readFirstCachedMessages(queryClient: QueryClient): MessageSummary[] {
  const entries = queryClient.getQueriesData<MessagesInfiniteData>({ queryKey: ["messages"] });
  for (const [, data] of entries) {
    const flat = data?.pages.flat() ?? [];
    if (flat.length > 0) return flat;
  }
  return [];
}

/**
 * Snapshot every ["messages", ...] cache entry so it can be restored on
 * optimistic-update rollback. Returns an array of [queryKey, data] pairs.
 */
export function snapshotMessagesCache(queryClient: QueryClient) {
  return queryClient.getQueriesData<MessagesInfiniteData>({ queryKey: ["messages"] });
}

export function restoreMessagesCache(
  queryClient: QueryClient,
  snapshot: ReturnType<typeof snapshotMessagesCache>,
) {
  for (const [key, data] of snapshot) {
    queryClient.setQueryData(key, data);
  }
}
