(use-modules (srfi srfi-1)
             (ice-9 pretty-print))

;; TODO: Move the below to a module.
;; TODO: Avoid hardcoding target/debug directory.
(let ()
  (load-extension "target/debug/libbats" "init_bats")
  (activate-logging)
  (ensure-init))

;; Example procedure.
(define (delete-all-tracks!)
  (count identity
         (map (lambda (t) (delete-track! (assoc-ref t 'id)))
              (tracks))))
