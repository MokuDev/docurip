export type CrawlProfile = 'apiDocs' | 'wiki' | 'blog' | 'documentation' | 'custom';

export interface CrawlProfileInfo {
  id: CrawlProfile;
  label: string;
  description: string;
  defaultMaxDepth: number;
  defaultPageLimit: number;
  defaultRespectRobotsTxt: boolean;
}

export const CRAWL_PROFILES: CrawlProfileInfo[] = [
  {
    id: 'apiDocs',
    label: 'API Documentation',
    description: 'Optimized for API reference docs with strict selectors',
    defaultMaxDepth: 3,
    defaultPageLimit: 500,
    defaultRespectRobotsTxt: true,
  },
  {
    id: 'wiki',
    label: 'Wiki',
    description: 'Broad crawl for wiki-style content',
    defaultMaxDepth: 4,
    defaultPageLimit: 2000,
    defaultRespectRobotsTxt: true,
  },
  {
    id: 'blog',
    label: 'Blog',
    description: 'Article-focused crawl, excludes comments and tags',
    defaultMaxDepth: 2,
    defaultPageLimit: 100,
    defaultRespectRobotsTxt: false,
  },
  {
    id: 'documentation',
    label: 'Documentation',
    description: 'Balanced defaults for general documentation sites',
    defaultMaxDepth: 3,
    defaultPageLimit: 1000,
    defaultRespectRobotsTxt: true,
  },
  {
    id: 'custom',
    label: 'Custom',
    description: 'Manual configuration',
    defaultMaxDepth: 2,
    defaultPageLimit: 1000,
    defaultRespectRobotsTxt: true,
  },
];

export interface CrawlConfig {
  url: string;
  maxDepth: number;
  pageLimit: number;
  downloadAssets: boolean;
  headlessStrategy: 'never' | 'auto' | 'always';
  contentSelectors: string[];
  excludePatterns: string[];
  includePatterns: string[];
  pathPrefix: string;
  respectRobotsTxt: boolean;
  stayWithinDomain: boolean;
  ssrfProtection: boolean;
  outputDir: string;
  profile?: CrawlProfile | null;
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

export type ThemePreference = 'dark' | 'light' | 'system';

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
  notificationsEnabled: boolean;
  theme: ThemePreference;
  shortcutOverrides: Record<string, string>;
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

export type ExportFormat = 'md_files' | 'pdf_files' | 'merged_md' | 'merged_pdf' | 'json_files' | 'merged_json' | 'html_files' | 'merged_html';

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
    format: 'json_files',
    label: 'JSON Files',
    description: 'Individual .json files per page',
    requiresHeadless: false,
  },
  {
    format: 'merged_json',
    label: 'Merged JSON',
    description: 'All pages in one JSON array',
    requiresHeadless: false,
  },
  {
    format: 'html_files',
    label: 'HTML Files',
    description: 'Individual .html files per page',
    requiresHeadless: false,
  },
  {
    format: 'merged_html',
    label: 'Merged HTML',
    description: 'All pages combined into one .html file',
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

export interface ImportResult {
  markdownPath: string;
  imagesDir: string | null;
  pageCount: number;
  imageCount: number;
  title: string;
}