/** @type {import('tailwindcss').Config} */
module.exports = {
    darkMode: ['class'],
    content: [
    './src/pages/**/*.{js,ts,jsx,tsx,mdx}',
    './src/components/**/*.{js,ts,jsx,tsx,mdx}',
    './src/app/**/*.{js,ts,jsx,tsx,mdx}',
  ],
  theme: {
  	extend: {
  		fontFamily: {
  			sans: ['var(--font-ui)'],
  			display: ['var(--font-display)'],
  			serif: ['var(--font-display)'],
  			mono: ['var(--font-mono)']
  		},
  		colors: {
  			background: 'hsl(var(--background))',
  			foreground: 'hsl(var(--foreground))',
  			border: 'hsl(var(--border))',
  			input: 'hsl(var(--input))',
  			ring: 'hsl(var(--ring))',
  			primary: {
  				DEFAULT: 'hsl(var(--primary))',
  				foreground: 'hsl(var(--primary-foreground))'
  			},
  			secondary: {
  				DEFAULT: 'hsl(var(--secondary))',
  				foreground: 'hsl(var(--secondary-foreground))'
  			},
  			tertiary: '#64748b',
  			// Brand ramps: warm paper greys, and slate ink wherever the app used blue or purple.
  			gray: { 50:'#f7f6f1',100:'#efece2',200:'#e6e1d3',300:'#d2cdbd',400:'#9c9d9a',500:'#6b6e75',600:'#565a63',700:'#3b4252',800:'#2b323d',900:'#232a35',950:'#171b22' },
  			slate: { 50:'#f7f6f1',100:'#efece2',200:'#e6e1d3',300:'#d2cdbd',400:'#9c9d9a',500:'#6b6e75',600:'#565a63',700:'#3b4252',800:'#2b323d',900:'#232a35',950:'#171b22' },
  			blue: { 50:'#f4f2ea',100:'#ebe8dd',200:'#d9d5c6',300:'#b9b6ab',400:'#8a8d96',500:'#5d6270',600:'#3b4252',700:'#2f3541',800:'#232a35',900:'#181d25',950:'#12161c' },
  			indigo: { 50:'#f4f2ea',100:'#ebe8dd',200:'#d9d5c6',300:'#b9b6ab',400:'#8a8d96',500:'#5d6270',600:'#3b4252',700:'#2f3541',800:'#232a35',900:'#181d25',950:'#12161c' },
  			purple: { 50:'#f4f2ea',100:'#ebe8dd',200:'#d9d5c6',300:'#b9b6ab',400:'#8a8d96',500:'#5d6270',600:'#3b4252',700:'#2f3541',800:'#232a35',900:'#181d25',950:'#12161c' },
  			violet: { 50:'#f4f2ea',100:'#ebe8dd',200:'#d9d5c6',300:'#b9b6ab',400:'#8a8d96',500:'#5d6270',600:'#3b4252',700:'#2f3541',800:'#232a35',900:'#181d25',950:'#12161c' },
  			card: {
  				DEFAULT: 'hsl(var(--card))',
  				foreground: 'hsl(var(--card-foreground))'
  			},
  			popover: {
  				DEFAULT: 'hsl(var(--popover))',
  				foreground: 'hsl(var(--popover-foreground))'
  			},
  			muted: {
  				DEFAULT: 'hsl(var(--muted))',
  				foreground: 'hsl(var(--muted-foreground))'
  			},
  			accent: {
  				DEFAULT: 'hsl(var(--accent))',
  				foreground: 'hsl(var(--accent-foreground))'
  			},
  			destructive: {
  				DEFAULT: 'hsl(var(--destructive))',
  				foreground: 'hsl(var(--destructive-foreground))'
  			},
  			chart: {
  				'1': 'hsl(var(--chart-1))',
  				'2': 'hsl(var(--chart-2))',
  				'3': 'hsl(var(--chart-3))',
  				'4': 'hsl(var(--chart-4))',
  				'5': 'hsl(var(--chart-5))'
  			}
  		},
  		borderRadius: {
  			lg: 'var(--radius)',
  			md: 'calc(var(--radius) - 2px)',
  			sm: 'calc(var(--radius) - 4px)'
  		},
  		keyframes: {
  			'accordion-down': {
  				from: {
  					height: '0'
  				},
  				to: {
  					height: 'var(--radix-accordion-content-height)'
  				}
  			},
  			'accordion-up': {
  				from: {
  					height: 'var(--radix-accordion-content-height)'
  				},
  				to: {
  					height: '0'
  				}
  			}
  		},
  		animation: {
  			'accordion-down': 'accordion-down 0.2s ease-out',
  			'accordion-up': 'accordion-up 0.2s ease-out'
  		}
  	}
  },
  plugins: [
    require("tailwindcss-animate"),
    require("@tailwindcss/typography"),
    require("@tailwindcss/container-queries"),
  ],
}