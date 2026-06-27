export interface CrawlConfig {
  url: string;
  maxDepth: number;
  pageLimit: number;
  downloadAssets: boolean;
  headlessStrategy: 'never' | 'auto' | 'always';
  contentSelectors: string[];
  excludePatterns: string[];
  respectRobotsTxt: boolean;
  stayWithinDomain: boolean;
  ssrfProtection: boolean;
  outputDir: string;
}

export interface PageMeta {
  url: string;
  title: string;
  status: number;
  linksCount: number;
}

/** Full page data returned only by read_page_content — not stored in CrawlJob. */
export interface PageResult {
  url: string;
  title: string;
  content: string;
  links: string[];
  assets: string[];
  status: number;
}

export interface CrawlProgress {
  pagesCrawled: number;
  pageLimit: number;
  currentUrl: string;
  depth: number;
  maxDepth: number;
  startTime: string;
}

export type JobStatus = 'queued' | 'running' | 'paused' | 'completed' | 'failed' | 'cancelled';

export type ErrorKind = 'network' | 'disk' | 'parse' | 'robotsBlocked' | 'cancelled' | 'unknown';

export interface CrawlJob {
  id: string;
  url: string;
  status: JobStatus;
  config: CrawlConfig;
  progress: CrawlProgress;
  results: PageMeta[];
  startTime?: string;
  endTime?: string;
  error?: string;
}

export interface AppSettings {
  outputDir: string;
  concurrency: number;
  requestDelay: number;
  timeout: number;
  userAgent: string;
  defaultMaxDepth: number;
  defaultPageLimit: number;
  defaultDownloadAssets: boolean;
  defaultHeadlessStrategy: string;
  defaultRespectRobotsTxt: boolean;
  defaultStayWithinDomain: boolean;
  defaultSsrfProtection: boolean;
  windowWidth: number;
  windowHeight: number;
}

export interface CrawlEvent {
  type: 'progress' | 'log' | 'pageComplete' | 'jobStatusChanged' | 'error';
  jobId: string;
  message?: string;
  level?: string;
  progress?: CrawlProgress;
  page?: PageMeta;
  status?: JobStatus;
  kind?: ErrorKind;
}

export interface SearchMatch {
  url: string;
  title: string;
  preview: string;
  relevance: number;
}

export interface DashboardStats {
  pagesSaved: number;
  totalSizeBytes: number;
  crawlVelocity: number;
  failRate: number;
}

export interface RecentExport {
  path: string;
  jobId: string;
  createdAt: string;
  sizeBytes: number;
}

export interface SystemStats {
  cpuPercent: number;
  memUsedMb: number;
  memTotalMb: number;
}

export interface SessionInfo {
  id: string;
  uptimeSecs: number;
}

export type ExportFormat = 'md_files' | 'pdf_files' | 'merged_md' | 'merged_pdf';

export interface ExportOption {
  format: ExportFormat;
  label: string;
  description: string;
  requiresHeadless: boolean;
}

export const EXPORT_OPTIONS: ExportOption[] = [
  {
    format: 'md_files',
    label: 'Markdown Files',
    description: 'Individual .md files in folder structure',
    requiresHeadless: false,
  },
  {
    format: 'merged_md',
    label: 'Merged Markdown',
    description: 'All pages combined into one .md file',
    requiresHeadless: false,
  },
  {
    format: 'pdf_files',
    label: 'PDF Files',
    description: 'Individual .pdf files per page',
    requiresHeadless: true,
  },
  {
    format: 'merged_pdf',
    label: 'Merged PDF',
    description: 'All pages in one PDF document',
    requiresHeadless: true,
  },
];