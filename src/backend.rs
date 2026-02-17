use std::sync::Mutex;

use dashmap::DashMap;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};
use tree_sitter::Parser;

use crate::definition;
use crate::diagnostics;
use crate::document::Document;
use crate::hover;
use crate::rename;
use crate::semantic_tokens::{self, LEGEND_TYPE};
use crate::symbols;

pub struct Backend {
    client: Client,
    documents: DashMap<Url, Document>,
    parser: Mutex<Parser>,
}

impl Backend {
    pub fn new(client: Client) -> Self {
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_rholang::LANGUAGE.into())
            .expect("Failed to load Rholang tree-sitter grammar");

        Backend {
            client,
            documents: DashMap::new(),
            parser: Mutex::new(parser),
        }
    }

    async fn publish_diagnostics(&self, uri: &Url) {
        if let Some(doc) = self.documents.get(uri) {
            let diags = diagnostics::collect_diagnostics(&doc);
            self.client
                .publish_diagnostics(uri.clone(), diags, None)
                .await;
        }
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                definition_provider: Some(OneOf::Left(true)),
                references_provider: Some(OneOf::Left(true)),
                document_symbol_provider: Some(OneOf::Left(true)),
                rename_provider: Some(OneOf::Right(RenameOptions {
                    prepare_provider: Some(true),
                    work_done_progress_options: WorkDoneProgressOptions::default(),
                })),
                semantic_tokens_provider: Some(
                    SemanticTokensServerCapabilities::SemanticTokensOptions(
                        SemanticTokensOptions {
                            legend: SemanticTokensLegend {
                                token_types: LEGEND_TYPE.to_vec(),
                                token_modifiers: vec![],
                            },
                            full: Some(SemanticTokensFullOptions::Bool(true)),
                            range: None,
                            work_done_progress_options: WorkDoneProgressOptions::default(),
                        },
                    ),
                ),
                ..Default::default()
            },
            server_info: Some(ServerInfo {
                name: "rholang-lsp".into(),
                version: Some(env!("CARGO_PKG_VERSION").into()),
            }),
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        log::info!("rholang-lsp initialized");
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri.clone();
        let source = params.text_document.text;
        {
            let mut parser = self.parser.lock().unwrap();
            if let Some(doc) = Document::new(&mut parser, source) {
                self.documents.insert(uri.clone(), doc);
            }
        }
        self.publish_diagnostics(&uri).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri.clone();
        if let Some(change) = params.content_changes.into_iter().last() {
            let mut parser = self.parser.lock().unwrap();
            if let Some(mut doc) = self.documents.remove(&uri) {
                doc.1.reparse(&mut parser, change.text);
                self.documents.insert(uri.clone(), doc.1);
            } else if let Some(doc) = Document::new(&mut parser, change.text) {
                self.documents.insert(uri.clone(), doc);
            }
        }
        self.publish_diagnostics(&uri).await;
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri;
        self.documents.remove(&uri);
        // Clear diagnostics
        self.client.publish_diagnostics(uri, vec![], None).await;
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = &params.text_document_position_params.text_document.uri;
        let pos = params.text_document_position_params.position;
        Ok(self.documents.get(uri).and_then(|doc| hover::hover(&doc, pos)))
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let uri = &params.text_document_position_params.text_document.uri;
        let pos = params.text_document_position_params.position;
        Ok(self.documents.get(uri).and_then(|doc| {
            definition::goto_definition(&doc, pos).map(|mut loc| {
                loc.uri = uri.clone();
                GotoDefinitionResponse::Scalar(loc)
            })
        }))
    }

    async fn references(&self, params: ReferenceParams) -> Result<Option<Vec<Location>>> {
        let uri = &params.text_document_position.text_document.uri;
        let pos = params.text_document_position.position;
        Ok(self.documents.get(uri).map(|doc| {
            definition::find_references(&doc, pos, uri)
        }).filter(|v| !v.is_empty()))
    }

    async fn document_symbol(
        &self,
        params: DocumentSymbolParams,
    ) -> Result<Option<DocumentSymbolResponse>> {
        let uri = &params.text_document.uri;
        Ok(self.documents.get(uri).map(|doc| {
            DocumentSymbolResponse::Nested(symbols::document_symbols(&doc))
        }))
    }

    async fn rename(&self, params: RenameParams) -> Result<Option<WorkspaceEdit>> {
        let uri = &params.text_document_position.text_document.uri;
        let pos = params.text_document_position.position;
        let new_name = params.new_name;
        Ok(self
            .documents
            .get(uri)
            .and_then(|doc| rename::rename(&doc, pos, new_name, uri)))
    }

    async fn prepare_rename(
        &self,
        params: TextDocumentPositionParams,
    ) -> Result<Option<PrepareRenameResponse>> {
        let uri = &params.text_document.uri;
        let pos = params.position;
        Ok(self
            .documents
            .get(uri)
            .and_then(|doc| rename::prepare_rename(&doc, pos)))
    }

    async fn semantic_tokens_full(
        &self,
        params: SemanticTokensParams,
    ) -> Result<Option<SemanticTokensResult>> {
        let uri = &params.text_document.uri;
        Ok(self.documents.get(uri).map(|doc| {
            SemanticTokensResult::Tokens(SemanticTokens {
                result_id: None,
                data: semantic_tokens::semantic_tokens(&doc),
            })
        }))
    }
}
