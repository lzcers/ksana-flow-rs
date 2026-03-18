import { defineNodeManifest } from '../nodeManifest';
import { ImgGenNode } from './index';
import { imgGenNodeMetadata } from './metadata';

export const imgGenNodeManifest = defineNodeManifest({
  metadata: imgGenNodeMetadata,
  Component: ImgGenNode,
  color: 'text-emerald-500 bg-emerald-50',
});
