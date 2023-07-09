(define-module (bats)
  #:use-module (srfi srfi-1)
  #:export (activate-logging!
            delete-all-tracks!
            delete-plugin-instance!
            delete-track!
            instantiate-plugin!
            instrument-plugins
            make-track!
            make-track-with-any-instrument!
            plugins
            settings
            track
            track-ids
            tracks
            ))

;; TODO: Move the below to a module.
;; TODO: Avoid hardcoding target/debug directory.
(load-extension "target/debug/libbats" "init_bats")

(define (delete-all-tracks!)
  "Delete all the tracks."
  (count identity
         (map delete-track! (track-ids))))

(define (instrument-plugins)
  "Get all plugins that are instruments."
  (filter (lambda (p) (assoc-ref p 'instrument?))
          (plugins)))

(define (tracks)
  "Get all tracks."
  (map track (track-ids)))
