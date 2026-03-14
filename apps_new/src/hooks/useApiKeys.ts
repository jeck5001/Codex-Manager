"use client";

import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { accountClient } from "@/lib/api/account-client";
import { toast } from "sonner";

export function useApiKeys() {
  const queryClient = useQueryClient();

  const apiKeysQuery = useQuery({
    queryKey: ["apikeys"],
    queryFn: () => accountClient.listApiKeys(),
  });

  const createMutation = useMutation({
    mutationFn: (params: any) => accountClient.createApiKey(params),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["apikeys"] });
      toast.success("密钥已创建");
    },
  });

  const deleteMutation = useMutation({
    mutationFn: (id: string) => accountClient.deleteApiKey(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["apikeys"] });
      toast.success("密钥已删除");
    },
  });

  const updateMutation = useMutation({
    mutationFn: ({ id, params }: { id: string; params: any }) => 
      accountClient.updateApiKey(id, params),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["apikeys"] });
      toast.success("密钥已更新");
    },
  });

  return {
    apiKeys: apiKeysQuery.data || [],
    isLoading: apiKeysQuery.isLoading,
    createApiKey: createMutation.mutate,
    deleteApiKey: deleteMutation.mutate,
    updateApiKey: updateMutation.mutate,
  };
}
