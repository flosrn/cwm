import { useParams, useNavigate } from "react-router-dom";
import { useClaudeProjects, useClaudeConfigFile, useWriteClaudeConfigFile } from "../../lib/query";
import { Alert, AlertDescription } from "../../components/ui/alert";
import { Button } from "../../components/ui/button";
import { ArrowLeftIcon, Check, ChevronsUpDown, FolderIcon, SaveIcon } from "lucide-react";
import { toast } from "sonner";
import CodeMirror, { EditorView, keymap } from '@uiw/react-codemirror';
import { json } from '@codemirror/lang-json';
import { useCodeMirrorTheme } from "../../lib/use-codemirror-theme";
import { codeFolding } from '@codemirror/language';
import { useState, useCallback, useEffect } from 'react';
import { useTranslation } from 'react-i18next';
import { Popover, PopoverContent, PopoverTrigger } from "../../components/ui/popover";
import { Command, CommandEmpty, CommandGroup, CommandInput, CommandItem, CommandList } from "../../components/ui/command";
import { cn } from "../../lib/utils";

export function Detail() {
  const { t } = useTranslation();
  const { path } = useParams<{ path: string }>();
  const navigate = useNavigate();
  const { data: projects, isLoading: isLoadingProjects, error: projectsError } = useClaudeProjects();
  const { data: claudeConfig, isLoading: isLoadingConfig, error: configError } = useClaudeConfigFile();
  const writeClaudeConfig = useWriteClaudeConfigFile();
  const [jsonContent, setJsonContent] = useState("");
  const [hasChanges, setHasChanges] = useState(false);
  const [comboboxOpen, setComboboxOpen] = useState(false);
  const codeMirrorTheme = useCodeMirrorTheme();

  // Update JSON content when project data loads or path changes
  useEffect(() => {
    if (path && projects && !isLoadingProjects && !isLoadingConfig) {
      const decodedPath = decodeURIComponent(path);
      const project = projects.find(p => p.path === decodedPath);

      if (project) {
        setJsonContent(JSON.stringify(project.config, null, 2));
        setHasChanges(false);
      }
    }
  }, [path, projects, isLoadingProjects, isLoadingConfig]);

  const handleSave = useCallback(() => {
    try {
      const parsedContent = JSON.parse(jsonContent);

      if (!path || !claudeConfig) {
        toast.error(t('projects.detail.noProjectSelected'));
        return;
      }

      const decodedPath = decodeURIComponent(path);
      const updatedConfig = JSON.parse(JSON.stringify(claudeConfig.content));

      // Update the specific project in the projects object
      if (!updatedConfig.projects) {
        updatedConfig.projects = {};
      }
      updatedConfig.projects[decodedPath] = parsedContent;

      writeClaudeConfig.mutate(updatedConfig);
      setHasChanges(false);
    } catch (error) {
      toast.error(t('projects.detail.invalidJson'));
    }
  }, [jsonContent, path, claudeConfig, writeClaudeConfig, t]);

  const handleContentChange = useCallback((value: string) => {
    setJsonContent(value);
    setHasChanges(true);
  }, []);

  const handleProjectChange = useCallback((newPath: string) => {
    if (hasChanges) {
      // Check if user wants to save before switching
      const shouldSave = window.confirm(t('projects.detail.unsavedChanges'));
      if (shouldSave) {
        handleSave();
      }
    }
    navigate(`/projects/${encodeURIComponent(newPath)}`);
  }, [hasChanges, handleSave, navigate, t]);

  // Create save keymap for CodeMirror
  const saveKeymap = keymap.of([
    {
      key: "Mod-s",
      run: () => {
        handleSave();
        return true; // Prevent further handling
      }
    }
  ]);

  // Create word wrap extension
  const wordWrapExtension = EditorView.lineWrapping;

  if (isLoadingConfig || isLoadingProjects) {
    return (
      <div className="">
        <div className="flex items-center p-3 border-b px-3 justify-between sticky top-0 bg-background z-10 mb-4" data-tauri-drag-region>
          <div data-tauri-drag-region>
            <h3 className="font-bold" data-tauri-drag-region>{t('projects.detail.editor')}</h3>
          </div>
        </div>
        <div className="space-y-6 px-4">
          <div className="flex items-center justify-center py-8">
            <div className="text-sm text-muted-foreground">{t('projects.detail.loading')}</div>
          </div>
        </div>
      </div>
    );
  }

  if (configError || projectsError) {
    return (
      <div className="w-full h-full flex items-center justify-center">
        <div className="space-y-6 px-4">
          {t('projects.detail.loadError')} {(configError || projectsError) instanceof Error ? (configError || projectsError)?.message : String(configError || projectsError)}
        </div>
      </div >
    );
  }

  if (!path || !projects || projects.length === 0) {
    return (
      <div className="w-full h-full flex items-center justify-center">
        <div className="space-y-6 px-4">
          {t('projects.detail.noProjectsMessage')}
        </div>
      </div>
    );
  }

  const decodedPath = decodeURIComponent(path);
  const project = projects.find(p => p.path === decodedPath);

  if (!project) {
    return (
      <div className="">
        <div className="flex items-center p-3 border-b px-3 justify-between sticky top-0 bg-background z-10 mb-4" data-tauri-drag-region>
          <div data-tauri-drag-region>
            <h3 className="font-bold" data-tauri-drag-region>{t('projects.detail.editor')}</h3>
          </div>
        </div>
        <div className="space-y-6 px-4">
          <Alert>
            <AlertDescription>
              {t('projects.detail.projectNotFound', { path: decodedPath })}
            </AlertDescription>
          </Alert>
          <Button onClick={() => navigate("/projects")} variant="outline">
            <ArrowLeftIcon className="h-4 w-4 mr-2" />
            {t('projects.detail.backToProjects')}
          </Button>
        </div>
      </div>
    );
  }

  return (
    <div className="flex flex-col h-screen">
      <div className="flex items-center px-4 justify-between py-3 border-b" data-tauri-drag-region>
        <div className="flex items-center gap-2">
          <h3 className="font-medium text-sm text-muted-foreground">{t('projects.detail.projectEditor')}</h3>
          <span className="text-muted-foreground text-xs">/</span>
          <Popover open={comboboxOpen} onOpenChange={setComboboxOpen}>
            <PopoverTrigger asChild>
              <Button
                variant="secondary"
                size="sm"
                role="combobox"
                aria-expanded={comboboxOpen}
                className="justify-between min-w-[200px]"
              >
                <span className="truncate">{project.path}</span>
                <ChevronsUpDown className="ml-2 h-4 w-4 shrink-0 opacity-50" />
              </Button>
            </PopoverTrigger>
            <PopoverContent className="w-[400px] p-0">
              <Command>
                <CommandInput placeholder={t('projects.detail.searchProject')} className="h-9" />
                <CommandList>
                  <CommandEmpty>{t('projects.detail.noProjectFound')}</CommandEmpty>
                  <CommandGroup>
                    {projects.map((proj) => (
                      <CommandItem
                        key={proj.path}
                        value={proj.path}
                        onSelect={() => {
                          handleProjectChange(proj.path);
                          setComboboxOpen(false);
                        }}
                      >
                        <FolderIcon className="mr-2 h-4 w-4" />
                        <span className="truncate">{proj.path}</span>
                        <Check
                          className={cn(
                            "ml-auto h-4 w-4",
                            project.path === proj.path ? "opacity-100" : "opacity-0"
                          )}
                        />
                      </CommandItem>
                    ))}
                  </CommandGroup>
                </CommandList>
              </Command>
            </PopoverContent>
          </Popover>
        </div>

        <div className="flex items-center gap-2">
          <Button
            onClick={handleSave}
            disabled={!hasChanges || writeClaudeConfig.isPending}
            size="sm"
            className="flex items-center gap-2"
          >
            <SaveIcon className="h-4 w-4" />
            {writeClaudeConfig.isPending ? t('projects.detail.saving') : t('projects.detail.save')}
          </Button>
        </div>
      </div>

      <div className="flex-1 overflow-hidden">
        <div className="overflow-hidden  h-full">
          <CodeMirror
            value={jsonContent}
            height="100%"
            theme={codeMirrorTheme}
            extensions={[json(), codeFolding(), wordWrapExtension, saveKeymap]}
            onChange={handleContentChange}
            basicSetup={{
              lineNumbers: true,
              highlightActiveLineGutter: true,
              foldGutter: true,
              dropCursor: false,
              allowMultipleSelections: false,
              indentOnInput: true,
              bracketMatching: true,
              closeBrackets: true,
              autocompletion: true,
              highlightActiveLine: true,
              highlightSelectionMatches: true,
              searchKeymap: true,
            }}
            className="h-full text-sm"
            style={{ width: '100%', fontSize: '12px' }}
          />
        </div>
      </div>
    </div>
  );
}