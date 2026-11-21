import {
  AddRounded,
  AltRouteRounded,
  RefreshRounded,
} from '@mui/icons-material'
import {
  Alert,
  Box,
  Button,
  Checkbox,
  FormControlLabel,
  IconButton,
  List,
  ListItem,
  Stack,
  TextField,
  Tooltip,
  Typography,
} from '@mui/material'
import { open } from '@tauri-apps/plugin-dialog'
import { useLockFn } from 'ahooks'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'

import { BaseDialog } from '@/components/base'
import { useVerge } from '@/hooks/use-verge'
import { listWindowsApps } from '@/services/cmds'
import { showNotice } from '@/services/notice-service'
import { useQuery } from '@/services/query-client'

type DialogProps = {
  initialPaths: string[]
  onClose: () => void
  onSave: (paths: string[]) => Promise<void>
}

const pathKey = (path: string) => path.replaceAll('/', '\\').toLowerCase()

const RuleDirectAppsDialog = ({
  initialPaths,
  onClose,
  onSave,
}: DialogProps) => {
  const { t } = useTranslation()
  const [selections, setSelections] = useState(
    () =>
      new Map(
        initialPaths.map((path) => [pathKey(path), { path, checked: true }]),
      ),
  )
  const [search, setSearch] = useState('')
  const [saving, setSaving] = useState(false)
  const [saveFailed, setSaveFailed] = useState(false)
  const {
    data: apps,
    error,
    isFetching,
    refetch,
  } = useQuery({
    queryKey: ['listWindowsApps'],
    queryFn: listWindowsApps,
    revalidateOnMount: true,
    refetchOnReconnect: false,
    retry: false,
  })
  const catalog = new Map(apps?.map((app) => [pathKey(app.path), app]))
  for (const [key, { path }] of selections) {
    if (!catalog.has(key)) {
      catalog.set(key, { name: path.split(/[/\\]/).at(-1) || path, path })
    }
  }
  const query = search.trim().toLowerCase()
  const queryPath = pathKey(query)
  const visibleApps = [...catalog.entries()]
    .filter(
      ([key, { name }]) =>
        key.includes(queryPath) || name.toLowerCase().includes(query),
    )
    .sort(
      ([, a], [, b]) =>
        a.name.localeCompare(b.name) || a.path.localeCompare(b.path),
    )
  const paths = [...selections.values()]
    .filter(({ checked }) => checked)
    .map(({ path }) => path)
  const changed =
    paths.length !== initialPaths.length ||
    paths.some((path, index) => path !== initialPaths[index])

  const setChecked = (paths: string[], checked: boolean) => {
    setSelections((current) => {
      const next = new Map(current)
      for (const path of paths) {
        const key = pathKey(path)
        next.set(key, { path: current.get(key)?.path ?? path, checked })
      }
      return next
    })
  }

  const addApps = useLockFn(async () => {
    try {
      const selected = await open({
        title: t('proxies.page.directApps.add'),
        multiple: true,
        filters: [{ name: 'EXE', extensions: ['exe'] }],
      })
      if (selected) setChecked(selected, true)
    } catch (error) {
      showNotice.error(error)
    }
  })

  const save = useLockFn(async () => {
    // A late core error can occur after the previous save took effect.
    if (!changed && !saveFailed) {
      onClose()
      return
    }
    setSaving(true)
    try {
      await onSave(paths)
      onClose()
    } catch (error) {
      setSaveFailed(true)
      showNotice.error(error)
    } finally {
      setSaving(false)
    }
  })

  return (
    <BaseDialog
      open
      title={t('proxies.page.directApps.title')}
      okBtn={t('shared.actions.save')}
      cancelBtn={t('shared.actions.cancel')}
      loading={saving}
      disableCancel={saving}
      onOk={() => void save()}
      onClose={saving ? undefined : onClose}
      onCancel={onClose}
      contentSx={{ width: { xs: 'min(480px, 75vw)', sm: 528 } }}
    >
      <Stack spacing={1.5}>
        <Typography variant="body2" color="text.secondary">
          {t('proxies.page.directApps.description')}
        </Typography>
        <Stack direction="row" spacing={1} sx={{ alignItems: 'center' }}>
          <TextField
            fullWidth
            label={t('proxies.page.directApps.search')}
            size="small"
            type="search"
            value={search}
            onChange={(event) => setSearch(event.target.value)}
          />
          <Tooltip title={t('shared.actions.refresh')}>
            <span>
              <IconButton
                aria-label={t('shared.actions.refresh')}
                disabled={saving}
                loading={isFetching}
                onClick={() => void refetch()}
              >
                <RefreshRounded />
              </IconButton>
            </span>
          </Tooltip>
        </Stack>
        <Stack
          direction="row"
          sx={{
            alignItems: 'center',
            justifyContent: 'space-between',
            gap: 1,
            flexWrap: 'wrap',
          }}
        >
          <Button
            size="small"
            variant="outlined"
            startIcon={<AddRounded />}
            disabled={saving}
            onClick={() => void addApps()}
          >
            {t('proxies.page.directApps.add')}
          </Button>
          <Typography variant="body2" color="text.secondary" aria-live="polite">
            {t('proxies.page.directApps.selected', { count: paths.length })}
          </Typography>
        </Stack>
        {error && (
          <Alert severity="warning">
            {t('proxies.page.directApps.loadFailed')}
          </Alert>
        )}
        {isFetching && !apps && (
          <Typography variant="body2" color="text.secondary" role="status">
            {t('shared.statuses.loading')}
          </Typography>
        )}
        {visibleApps.length > 0 ? (
          <List
            dense
            disablePadding
            sx={{ maxHeight: 'min(44vh, 380px)', overflowY: 'auto' }}
          >
            {visibleApps.map(([key, { name, path }]) => (
              <ListItem key={key} divider disablePadding>
                <FormControlLabel
                  disabled={saving}
                  disableTypography
                  sx={{
                    width: '100%',
                    m: 0,
                    py: 0.75,
                    pr: 1,
                    alignItems: 'flex-start',
                    '&:hover': { bgcolor: 'action.hover' },
                  }}
                  control={
                    <Checkbox
                      size="small"
                      checked={selections.get(key)?.checked ?? false}
                      onChange={(_, checked) => setChecked([path], checked)}
                    />
                  }
                  label={
                    <Box
                      component="span"
                      sx={{ overflowWrap: 'anywhere', minWidth: 0 }}
                    >
                      <Typography
                        component="span"
                        variant="body2"
                        sx={{ display: 'block' }}
                      >
                        {name}
                      </Typography>
                      <Typography
                        component="span"
                        variant="caption"
                        color="text.secondary"
                        sx={{ display: 'block' }}
                      >
                        {path}
                      </Typography>
                    </Box>
                  }
                />
              </ListItem>
            ))}
          </List>
        ) : (
          !isFetching &&
          (!error || catalog.size > 0) && (
            <Typography variant="body2" color="text.secondary">
              {query
                ? t('proxies.page.directApps.noResults')
                : t('proxies.page.directApps.empty')}
            </Typography>
          )
        )}
      </Stack>
    </BaseDialog>
  )
}

export const RuleDirectAppsButton = () => {
  const { t } = useTranslation()
  const { verge, patchVerge } = useVerge()
  const [initialPaths, setInitialPaths] = useState<string[] | null>(null)
  const title = t('proxies.page.directApps.title')

  return (
    <>
      <Tooltip title={title}>
        <span>
          <Button
            size="small"
            variant="outlined"
            aria-label={title}
            disabled={!verge}
            onClick={() => setInitialPaths(verge?.rule_direct_apps ?? [])}
            sx={{ gap: 0.5, minWidth: 32, whiteSpace: 'nowrap' }}
          >
            <AltRouteRounded fontSize="small" />
            <Box
              component="span"
              sx={{ '@media (max-width: 700px)': { display: 'none' } }}
            >
              {title}
            </Box>
          </Button>
        </span>
      </Tooltip>
      {initialPaths !== null && (
        <RuleDirectAppsDialog
          initialPaths={initialPaths}
          onClose={() => setInitialPaths(null)}
          onSave={(paths) => patchVerge({ rule_direct_apps: paths })}
        />
      )}
    </>
  )
}
