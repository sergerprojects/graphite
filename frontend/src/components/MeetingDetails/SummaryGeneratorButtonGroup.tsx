"use client";

import { ModelConfig, ModelSettingsModal } from '@/components/ModelSettingsModal';
import {
  Dialog,
  DialogContent,
  DialogTitle,
} from "@/components/ui/dialog"
import { VisuallyHidden } from "@/components/ui/visually-hidden"
import { Button } from '@/components/ui/button';
import { ButtonGroup } from '@/components/ui/button-group';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuSub,
  DropdownMenuSubContent,
  DropdownMenuSubTrigger,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import { Sparkles, Loader2, Check, Square, ChevronDown, Wand2 } from 'lucide-react';
import Analytics from '@/lib/analytics';
import { useState, useEffect } from 'react';

interface SummaryGeneratorButtonGroupProps {
  modelConfig: ModelConfig;
  setModelConfig: (config: ModelConfig | ((prev: ModelConfig) => ModelConfig)) => void;
  onSaveModelConfig: (config?: ModelConfig) => Promise<void>;
  onGenerateSummary: (customPrompt: string) => Promise<void>;
  onStopGeneration: () => void;
  customPrompt: string;
  summaryStatus: 'idle' | 'processing' | 'summarizing' | 'regenerating' | 'completed' | 'error';
  availableTemplates: Array<{ id: string, name: string, description: string }>;
  selectedTemplate: string;
  onTemplateSelect: (templateId: string, templateName: string) => void;
  hasTranscripts?: boolean;
  hasSummary?: boolean;
  isModelConfigLoading?: boolean;
  onOpenModelSettings?: (openFn: () => void) => void;
  /** Name of the summary language now in use ("Auto" when it follows the transcript). */
  languageLabel: string;
  onOpenLanguage: () => void;
  /** Open the "update from my edits" dialog. Only offered once there is a summary to update. */
  onUpdateFromEdits?: () => void;
  /** Write any unsaved edits before the summary is redone. */
  flushEdits?: () => Promise<void>;
}

/**
 * One main action ("Regenerate" or "Generate summary") with a menu beside it. The menu holds the
 * other way to redo the notes (update from my edits) and the settings that only matter when it runs:
 * template, language and model.
 */
export function SummaryGeneratorButtonGroup({
  modelConfig,
  setModelConfig,
  onSaveModelConfig,
  onGenerateSummary,
  onStopGeneration,
  customPrompt,
  summaryStatus,
  availableTemplates,
  selectedTemplate,
  onTemplateSelect,
  hasTranscripts = true,
  hasSummary = false,
  isModelConfigLoading = false,
  onOpenModelSettings,
  languageLabel,
  onOpenLanguage,
  onUpdateFromEdits,
  flushEdits,
}: SummaryGeneratorButtonGroupProps) {
  const [settingsDialogOpen, setSettingsDialogOpen] = useState(false);

  // Let the page open the model settings (for example when no model is set up yet).
  useEffect(() => {
    onOpenModelSettings?.(() => setSettingsDialogOpen(true));
  }, [onOpenModelSettings]);

  if (!hasTranscripts) {
    return null;
  }

  const isGenerating = summaryStatus === 'processing' || summaryStatus === 'summarizing' || summaryStatus === 'regenerating';
  const templateName = availableTemplates.find(t => t.id === selectedTemplate)?.name;

  return (
    <>
      <ButtonGroup>
        {isGenerating ? (
          <Button
            variant="outline"
            size="sm"
            className="bg-red-50 hover:bg-red-100 border-red-200 px-3 gap-2"
            onClick={() => {
              Analytics.trackButtonClick('stop_summary_generation', 'meeting_details');
              onStopGeneration();
            }}
            title="Stop summary generation"
          >
            <Square size={18} fill="currentColor" />
            <span>Stop</span>
          </Button>
        ) : (
          <Button
            variant="outline"
            size="sm"
            className="px-3 gap-2"
            onClick={() => {
              Analytics.trackButtonClick('generate_summary', 'meeting_details');
              void (async () => {
                try {
                  await flushEdits?.();
                } catch {
                  // The edits stay in the editor; still do what was asked.
                }
                await onGenerateSummary(customPrompt);
              })();
            }}
            disabled={isModelConfigLoading}
            title={
              isModelConfigLoading
                ? 'Loading model configuration...'
                : hasSummary
                  ? 'Write the summary again from the transcript. Your edits to it are replaced; Update from my edits keeps them.'
                  : 'Write the summary from the transcript'
            }
          >
            {isModelConfigLoading ? (
              <>
                <Loader2 className="animate-spin" size={18} />
                <span>Loading…</span>
              </>
            ) : (
              <>
                <Sparkles size={18} />
                <span>{hasSummary ? 'Regenerate' : 'Generate summary'}</span>
              </>
            )}
          </Button>
        )}

        <DropdownMenu modal={false}>
          <DropdownMenuTrigger asChild>
            <Button variant="outline" size="sm" aria-label="Summary options" title="Update from my edits, template, language, model">
              <ChevronDown size={16} />
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="start" className="w-72">
            {hasSummary && onUpdateFromEdits && !isGenerating && (
              <>
                <DropdownMenuItem onSelect={onUpdateFromEdits} className="gap-2">
                  <Wand2 size={16} />
                  <div>
                    <div>Update from my edits…</div>
                    <div className="text-xs text-gray-500">Rewrites the notes and keeps what you changed</div>
                  </div>
                </DropdownMenuItem>
                <DropdownMenuSeparator />
              </>
            )}
            <DropdownMenuLabel className="text-xs font-medium uppercase tracking-wide text-gray-500">
              Used when the summary is written
            </DropdownMenuLabel>
            {availableTemplates.length > 0 && (
              <DropdownMenuSub>
                <DropdownMenuSubTrigger className="justify-between gap-3">
                  <span>Template</span>
                  <span className="ml-auto truncate text-xs text-gray-500">{templateName ?? ''}</span>
                </DropdownMenuSubTrigger>
                <DropdownMenuSubContent>
                  {availableTemplates.map(template => (
                    <DropdownMenuItem
                      key={template.id}
                      onClick={() => onTemplateSelect(template.id, template.name)}
                      title={template.description}
                      className="flex items-center justify-between gap-2"
                    >
                      <span>{template.name}</span>
                      {selectedTemplate === template.id && <Check className="h-4 w-4 text-green-600" />}
                    </DropdownMenuItem>
                  ))}
                </DropdownMenuSubContent>
              </DropdownMenuSub>
            )}
            <DropdownMenuItem onSelect={onOpenLanguage} className="justify-between gap-3">
              <span>Language</span>
              <span className="truncate text-xs text-gray-500">{languageLabel}</span>
            </DropdownMenuItem>
            <DropdownMenuItem onSelect={() => setSettingsDialogOpen(true)} className="justify-between gap-3">
              <span>AI model</span>
              <span className="truncate text-xs text-gray-500">{modelConfig.model ?? 'Not set'}</span>
            </DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>
      </ButtonGroup>

      <Dialog open={settingsDialogOpen} onOpenChange={setSettingsDialogOpen}>
        <DialogContent aria-describedby={undefined}>
          <VisuallyHidden>
            <DialogTitle>Model Settings</DialogTitle>
          </VisuallyHidden>
          <ModelSettingsModal
            onSave={async (config) => {
              await onSaveModelConfig(config);
              setSettingsDialogOpen(false);
            }}
            modelConfig={modelConfig}
            setModelConfig={setModelConfig}
            skipInitialFetch={true}
            layout="dialog"
          />
        </DialogContent>
      </Dialog>
    </>
  );
}
