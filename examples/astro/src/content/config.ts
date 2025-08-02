import { defineCollection, z } from 'astro:content'

const blog = defineCollection({
  type: 'content',
  schema: z.object({
    title: z.string(),
    date: z.date(),
    description: z.string(),
    author: z.string().default('MDX Hybrid Team'),
    tags: z.array(z.string()).optional(),
  }),
})

export const collections = { blog }
