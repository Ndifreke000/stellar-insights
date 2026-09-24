import {   useQueryClient } from '@tanstack/react-query';
import { queryKeys, useApiQuery, useApiMutation } from '@/lib/react-query/hooks';
import {
  getSep24Info,
  startDepositInteractive,
  startWithdrawInteractive,
  getSep24Transactions,
  getSep24Anchors,
} from '@/services/sep24';
import { useAppStore } from '@/lib/zustand/store';

export function useSep24Anchors() {
  return useApiQuery(
    queryKeys.sep24Anchors,
    getSep24Anchors,
    {
      staleTime: 10 * 60 * 1000, // 10 minutes
      retry: 2,
    }
  );
}

export function useSep24Info(transferServer: string) {
  return useApiQuery(
    queryKeys.sep24Info(transferServer),
    () => getSep24Info(transferServer),
    {
      enabled: !!transferServer,
      staleTime: 5 * 60 * 1000, // 5 minutes
      retry: 2,
    }
  );
}

export function useSep24Transactions(transferServer: string, jwt?: string) {
  return useApiQuery(
    queryKeys.sep24Transactions(transferServer),
    () => getSep24Transactions({
      transfer_server: transferServer,
      jwt: jwt || undefined,
      limit: 20,
    }),
    {
      enabled: !!transferServer,
      staleTime: 30 * 1000, // 30 seconds
      retry: 2,
    }
  );
}

export function useStartDepositFlow() {
  const queryClient = useQueryClient();
  const { addNotification } = useAppStore();

  return useApiMutation(
    ({ transferServer, assetCode, amount, account, jwt }: {
      transferServer: string;
      assetCode?: string;
      amount?: string;
      account?: string;
      jwt?: string;
    }) => startDepositInteractive({
      transfer_server: transferServer,
      asset_code: assetCode,
      amount: amount || undefined,
      account: account || undefined,
      jwt: jwt || undefined,
    }),
    {
      onSuccess: (data, variables) => {
        // Open interactive URL in new window
        const url = data.url;
        if (url && url.startsWith('http')) {
          window.open(url, 'sep24-interactive', 'width=500,height=700');

          addNotification({
            type: 'success',
            message: 'Deposit flow started. Complete the flow in the popup window.',
          });
        }
        
        // Refresh transactions
        queryClient.invalidateQueries({
          queryKey: queryKeys.sep24Transactions(variables.transferServer),
        });
      },
      onError: (error) => {
        addNotification({
          type: 'error',
          message: `Failed to start deposit flow: ${error.message}`,
        });
      },
    }
  );
}

export function useStartWithdrawFlow() {
  const queryClient = useQueryClient();
  const { addNotification } = useAppStore();

  return useApiMutation(
    ({ transferServer, assetCode, amount, account, jwt }: {
      transferServer: string;
      assetCode?: string;
      amount?: string;
      account?: string;
      jwt?: string;
    }) => startWithdrawInteractive({
      transfer_server: transferServer,
      asset_code: assetCode,
      amount: amount || undefined,
      account: account || undefined,
      jwt: jwt || undefined,
    }),
    {
      onSuccess: (data, variables) => {
        // Open interactive URL in new window
        const url = data.url;
        if (url && url.startsWith('http')) {
          window.open(url, 'sep24-interactive', 'width=500,height=700');

          addNotification({
            type: 'success',
            message: 'Withdrawal flow started. Complete the flow in the popup window.',
          });
        }
        
        // Refresh transactions
        queryClient.invalidateQueries({
          queryKey: queryKeys.sep24Transactions(variables.transferServer),
        });
      },
      onError: (error) => {
        addNotification({
          type: 'error',
          message: `Failed to start withdrawal flow: ${error.message}`,
        });
      },
    }
  );
}

export function useSep24FlowState() {
  const {
    formData,
    setFormData,
    clearFormData,
    setFormErrors,
    clearFormErrors,
    setLoading,
    loading,
  } = useAppStore();

  const formKey = 'sep24-flow';
  
  const setFormDataValue = (key: string, value: unknown) => {
    setFormData(formKey, { ...formData[formKey], [key]: value });
  };

  const clearFormDataValue = (key?: string) => {
    if (key) {
      const currentData = formData[formKey] || {};
      const newData = { ...currentData };
      delete newData[key];
      setFormData(formKey, newData);
    } else {
      clearFormData(formKey);
    }
  };

  return {
    formData: formData[formKey] || {},
    setFormDataValue,
    clearFormDataValue,
    setFormErrors,
    clearFormErrors,
    setLoading,
    loading,
  };
}
