const paths = {
  folder: 'M3 7V5a1 1 0 0 1 1-1h5l2 3h9a1 1 0 0 1 1 1v11a1 1 0 0 1-1 1H4a1 1 0 0 1-1-1V7Z',
  branch: 'M6 3v12m12-9v3a6 6 0 0 1-6 6H6M3 18a3 3 0 1 0 6 0 3 3 0 0 0-6 0ZM15 3a3 3 0 1 0 6 0 3 3 0 0 0-6 0Z',
  download: 'M12 3v12m-5-5 5 5 5-5M4 16v4h16v-4',
  upload: 'M12 16V4m-5 5 5-5 5 5M4 16v4h16v-4',
  server: 'M4 3h16v7H4V3Zm0 11h16v7H4v-7Zm3-7h.01M7 18h.01M11 7h6m-6 11h6',
  history: 'M3 11a9 9 0 1 1 2 7M3 4v7h7m2-4v5l3 2',
  settings: 'M4 7h16M4 17h16M8 4v6m8 4v6',
  plus: 'M12 5v14M5 12h14',
  check: 'm5 12 4 4L19 6',
  close: 'm6 6 12 12M6 18 18 6',
  arrow: 'M4 12h16m-6-6 6 6-6 6',
  file: 'M14 3H5v18h14V8l-5-5Zm0 0v6h5M8 13h8m-8 4h5'
};

export function Icon({ name, className = '' }: { name: keyof typeof paths; className?: string }) {
  return (
    <svg
      className={`icon ${className}`}
      width="20"
      height="20"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.6"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      <path d={paths[name]} />
    </svg>
  );
}
