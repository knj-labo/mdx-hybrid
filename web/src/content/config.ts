import { defineCollection, z } from 'astro:content'

const docs = defineCollection({
  type: 'content',
  schema: z.object({
    title: z.string(),
    description: z.string().max(160).optional(),
    pubDate: z.coerce.date().optional(),
    draft: z.boolean().default(false),
  }),
})

export const collections = { docs }
